---
name: nexa-framework
description: "Use when changing the Nexa Rust compiler, language, typed IR, CLI, native Swift or Kotlin code generators, core UI components, build workflow, or performance architecture."
---

# Nexa Framework Development

Work on the Nexa framework itself: a Rust ahead-of-time compiler that turns `.nx` app source into native SwiftUI or Jetpack Compose code. Read the repository `README.md`, `plan.md`, and the relevant language docs before changing architecture or syntax. Treat the implementation as an early prototype; do not describe roadmap items as implemented.

## Architecture boundaries

- Keep the source pipeline separated into syntax, semantic analysis, platform-independent IR, backend code generation, and CLI responsibilities. Put each responsibility in its existing crate or a focused module under it.
- Keep the authoring surface small: canonicalize `Column` to the existing vertical `View` layout during parsing, and infer only immutable `let` declarations when a non-empty initializer has one unambiguous type. Mutable `state` and component parameters stay explicit. These are compiler-only conveniences and must preserve generated native output for equivalent typed source.
- Run conservative IR optimizations after semantic lowering and before either backend. Fold only pure literal expressions and remove statically unreachable UI/action branches; preserve short-circuit behavior, state semantics, and native component mappings. Keep the optimization pass in `nexa-compiler` so backend code remains platform-focused.
- Keep the common IR platform-independent. Swift-specific generation belongs in `nexa-backend-swift`; Kotlin/Compose-specific generation belongs in `nexa-backend-kotlin`.
- Keep compiler/code-generation responsibilities in separate files and focused subfolders. Prefer small modules with clear ownership over a monolithic file, while avoiding abstractions that add indirection without reuse.
- Add core default components to Nexa's shared language/IR and both native backends when requested. Optional integrations such as SQLite, MMKV, and maps belong in a future plugin layer; do not add them to core by default.
- Keep the authored app in `.nx`. Generated Swift/Kotlin is compiler output; do not ask app authors to write native code to use supported Nexa features.

## Native output and performance

- Generate ordinary native SwiftUI and Kotlin/Jetpack Compose code with direct platform calls and minimal framework machinery. Preserve platform behavior, accessibility, and native state semantics.
- Prioritize predictable allocations and direct native primitives. Make performance claims only when supported by generated output or measurements; do not label the prototype native-equivalent without benchmarks.
- Keep module-wide code-generation scans single-pass and share recursive IR traversal across backends. Derive imports and optional native helpers from analyzed features; never emit broad wildcard imports by default.
- Emit each reachable user component once and call it directly for every use. Do not duplicate its view body at call sites or add a component registry/runtime; keep generated component declarations out of the app's public API.
- Android image nodes must use Coil 3 (`coil3.compose.AsyncImage`) and keep image-specific code isolated in the Kotlin image generator.
- Theme values belong in typed, platform-independent IR and should resolve statically when possible. Emit system appearance lookups only for apps that use adaptive color tokens; keep typography, spacing, radius, and other token behavior native to each backend.
- User-defined components should lower to typed IR and ordinary native view/composable declarations. Resolve relative `.nx` imports at compile time, preserve component-local state per native instance, reject recursive composition, and prune unreachable generated components; avoid runtime registries and duplicated UI trees.
- Conditions and scalar operators should be type-checked before code generation and lower directly to Swift/Kotlin operators and native control flow. Preserve short-circuit semantics and avoid implicit numeric conversions or an interpreted expression runtime.
- Preserve the core value mappings for `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>` in the typed IR and both backends. Keep literals type-directed, map duplicate keys last-value-wins, and Set/Map hash keys scalar-only until compatible native constraints are modeled. Use Swift tuples for Pair/Triple and Kotlin standard-library Pair/Triple, without generated helper wrappers. These values have no source-level indexing, lookup, mutation, general iteration, or transformation API; `FastList` iterates arrays for UI rows. Update the language guide, design decisions, examples, and this skill together when that boundary changes.
- iOS `FastList` should use native `UITableView` virtualization and cell reuse; host SwiftUI row content with `UIHostingConfiguration` instead of routing large lists through SwiftUI `List`. Keep row lookup direct and avoid materializing a second row collection.
- Keep component generation separate by concern (controls, text input, image loading, layout, navigation, lists, keyboard handling, expressions, and shared formatting helpers).
- Lower shared layout alignment to native stack constructor arguments; avoid adding wrapper layouts solely to position children. Keep common style-property predicates in the shared IR to prevent backend feature scans from drifting.
- Keep `Layout.isRegularWidth` as a typed responsive predicate with feature-gated native environment access: Swift horizontal size class and Compose configuration width `>= 600dp`. Document the platform-specific meaning and keep the broader breakpoint/device-class API on the roadmap until implemented.
- Preserve FastList as the public component name. Do not reintroduce the former UltraFastList name.

## Change workflow

1. Trace the feature from lexer/parser through semantic checks and IR into each affected backend.
2. Keep diagnostics source-aware and reject unsupported or ill-typed syntax before code generation.
3. Update focused language docs and examples when changing user-visible syntax or component behavior.
4. Run the narrowest relevant compiler/build checks. Inspect generated Swift/Kotlin for platform correctness; a successful Rust build alone does not prove generated native code compiles in a host app.
5. Avoid adding dependencies or runtime layers unless the feature requires them and their cost is clear.
