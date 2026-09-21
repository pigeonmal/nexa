---
name: nexa-app
description: "Use when helping a Nexa user plan, author, structure, or troubleshoot a mobile app in `.nx`, especially when the user wants a complete app without writing Swift or Kotlin."
---

# Create an App with Nexa

Help people build an iOS and Android app from Nexa's shared `.nx` source without asking them to edit native Swift or Kotlin. Start from the user's product goal, clarify important screens and interactions only when needed, and build a coherent app using the framework's current core components.

## Work within today's language

- Read `README.md`, `docs/language.md`, and the examples relevant to the request before writing `.nx` syntax. The language and compiler are an early prototype; check actual parser/backend support instead of assuming the roadmap is implemented.
- Use core components already supported by Nexa, such as Column, Row, Text, Button, TextInput, Switch, Image, navigation, keyboard-aware layout, and FastList when available in the current compiler.
- Use `if`/`else`, `&&`, `||`, `!`, scalar `==`/`!=`, and numeric comparisons for supported conditional UI and button/press actions. Keep compared numeric types equal; Nexa does not implicitly convert values.
- Use `$name` or `\(name)` inside strings when a label or message includes a declared state or constant. Interpolation is compiled directly to native Swift/Kotlin string interpolation; only named values are supported inside the string for now.
- Pure literal conditions are folded by the compiler, so unreachable UI and event branches are removed from generated native source. Conditions that read state or platform environment remain native runtime branches.
- The compiler reports unused states, `let` constants, component parameters, and constant conditions. Use `--deny-warnings` when warnings should fail a check or build; see `docs/compiler-diagnostics.md` for the policy.
- Use `platform ios { ... }` and `platform android { ... }` for widgets that should exist on one target only. `nexa check` validates both target branches; `nexa build --target swift|kotlin` emits only the selected branch, with no runtime platform check.
- Remote `Image(url: ...)` requests use Nexa's generated native network stack. iOS uses URLSession; Android uses Coil 3 through Cronet with HTTP/2, QUIC, Brotli, and a 64 MiB disk cache. Add the generated app's documented URLSession/Cronet dependencies and do not add Coil's OkHttp network module.
- The generated native library exposes asynchronous `NexaNetwork.fetch`/`download`, `NexaPath`, and `NexaFile` helpers. Request options include method, body, headers, timeout, cache use, redirects, response size limits, and optional certificate pins; the current `.nx` slice does not yet have first-class async call syntax.
- Use `Column` as the vertical container and `Row` for horizontal layout. Set a layout's cross-axis `alignment` to `Start`, `Center`, or `End` where needed; vertical layouts align horizontally, while rows align vertically.
- Configure the app status bar with one top-level `StatusBar(style: Default|Light|Dark, hidden: true|false)` declaration. `style` describes status-bar content; nested and repeated declarations are rejected.
- Use `BottomSheet(isPresented: mutableBool) { ... }` for native modal content. The sheet closes by setting the bound Boolean to `false`; advanced snap points and custom transitions are not part of the current syntax.
- Use `RefreshControl(isRefreshing: mutableBool) { ... } { ... }` for native pull-to-refresh. Put direct state assignments in the second block; custom indicators and refresh streams are not part of the current syntax.
- Use `AppBottomBar(selected: mutableInt32) { Tab(index: 0, label: "Home") { ... } ... }` for native labeled tabs. Indexes must be unique, non-negative `Int32` literals; Swift uses `TabView` and Android uses Material 3 `Scaffold`/`NavigationBar`. Icons, badges, and custom transitions are not part of the current syntax.
- Use `Link(url: "https://example.com") { ... }` to open a static web URL or application scheme with the native handler. Swift emits `Link`; Android emits an `ACTION_VIEW` intent after checking for a handler. Dynamic URLs, incoming links, and universal-link routing are not part of the current syntax.
- Use `Accessibility(label: "...", role: Button|Link|Header|Image) { ... }` to give a child subtree a static native accessibility label and role. Swift emits accessibility modifiers; Android emits Compose semantics and heading semantics for `Header`. Hints, focus control, and custom actions are not part of the current syntax.
- Use `Layout.isRegularWidth` in `if`/`else` when a wider composition is useful. It maps to Swift's regular horizontal size class and to Compose's current configuration width of at least `600dp`; document the platform difference when it affects a design.
- Keep user-authored application code in `.nx`. Both `state` and immutable `let` values can omit the type when their non-empty initializer is unambiguous; integer literals default to `Int32` and decimal literals to `Float64`. Use an explicit type for narrower numeric bindings or empty collections. Do not put app features in generated `.swift` or `.kt` files and do not make native edits a prerequisite for supported functionality.
- Use FastList for large or repeating collections when supported. Keep layout and state simple so generated code remains direct and efficient.
- Use the built-in typed values `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>` with declared types and contextual literals: `[a, b]` for arrays/sets, `[key: value]` for maps, `Pair(a, b)`, and `Triple(a, b, c)`. Set elements and map keys must be scalar `String`, `Bool`, or numeric values. These values can be declared, passed, assigned as whole values, and displayed; source-level indexing, lookup, membership, mutation, general iteration, and transformations are not supported yet. Set and map order is not cross-platform stable. Swift pairs/triples are tuples; Kotlin uses ordinary standard-library Pair/Triple objects, so avoid rebuilding them repeatedly in hot Compose paths. Use `FastList` to render arrays as repeated rows.
- Define reusable `component Name(property: Type)` declarations in `.nx`, with private `state` declarations and a `body`; invoke them by name using all required named properties. Each instance owns native component state.
- Split larger apps into `.nx` files with `import "relative/path/Component.nx"`. Resolve imports relative to the importing file, keep one project-wide component name scope, and put the single `app` declaration in the entry file. Imported component files may import other component files.
- Keep component boundaries focused. Callbacks, content slots, and `NavigationLink` from inside custom components are not supported yet; pass values as typed inputs and keep interactions inside the component when possible.
- For Android images, rely on Nexa's Coil 3 based image generation; explain any host-project Coil 3 setup that the current CLI does not generate.
- Optional plugins such as SQLite, MMKV, and maps are future extensions unless the repository currently supplies them. Do not invent plugin syntax or silently replace missing support with native code.

## User experience and accuracy

- Translate nontechnical requests into screens, state, navigation, and component behavior. Explain the proposed structure in plain language and preserve the requested visual/product intent.
- When a requested feature is unsupported, identify the specific gap and offer the closest working core-only approach. Do not claim a complete build, plugin installation, or app-store-ready project if Nexa currently only emits source files.
- Keep app source modular as it grows: separate screens and reusable component declarations using only syntax the language actually supports. Avoid premature plugin or runtime architecture in app code.
- Generate Swift and Kotlin from the same `.nx` source and check that both targets express equivalent behavior. Prefer platform-native APIs through the compiler's existing mappings.
- Give clear build instructions using the current Nexa CLI and describe any required host Xcode/Android project setup without asking the user to write native source for features the language supports.
