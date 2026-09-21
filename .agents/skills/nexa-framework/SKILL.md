---
name: nexa-framework
description: "Use when changing the Nexa Rust compiler, language, typed IR, CLI, native Swift or Kotlin code generators, core UI components, build workflow, or performance architecture."
---

# Nexa Framework Development

Work on the Nexa framework itself: a Rust ahead-of-time compiler that turns `.nx` app source into native SwiftUI or Jetpack Compose code. Read the repository `README.md`, `plan.md`, and the relevant language docs before changing architecture or syntax. Treat the implementation as an early prototype; do not describe roadmap items as implemented.

## Architecture boundaries

- Keep the source pipeline separated into syntax, semantic analysis, platform-independent IR, backend code generation, and CLI responsibilities. Put each responsibility in its existing crate or a focused module under it.
- Keep the common IR platform-independent. Swift-specific generation belongs in `nexa-backend-swift`; Kotlin/Compose-specific generation belongs in `nexa-backend-kotlin`.
- Keep compiler/code-generation responsibilities in separate files and focused subfolders. Prefer small modules with clear ownership over a monolithic file, while avoiding abstractions that add indirection without reuse.
- Add core default components to Nexa's shared language/IR and both native backends when requested. Optional integrations such as SQLite, MMKV, and maps belong in a future plugin layer; do not add them to core by default.
- Keep the authored app in `.nx`. Generated Swift/Kotlin is compiler output; do not ask app authors to write native code to use supported Nexa features.

## Native output and performance

- Generate ordinary native SwiftUI and Kotlin/Jetpack Compose code with direct platform calls and minimal framework machinery. Preserve platform behavior, accessibility, and native state semantics.
- Prioritize predictable allocations and direct native primitives. Make performance claims only when supported by generated output or measurements; do not label the prototype native-equivalent without benchmarks.
- Android image nodes must use Coil 3 (`coil3.compose.AsyncImage`) and keep image-specific code isolated in the Kotlin image generator.
- iOS `FastList` should use native `UITableView` virtualization and cell reuse; host SwiftUI row content with `UIHostingConfiguration` instead of routing large lists through SwiftUI `List`. Keep row lookup direct and avoid materializing a second row collection.
- Keep component generation separate by concern (controls, text input, image loading, layout, navigation, lists, keyboard handling, expressions, and shared formatting helpers).
- Preserve FastList as the public component name. Do not reintroduce the former UltraFastList name.

## Change workflow

1. Trace the feature from lexer/parser through semantic checks and IR into each affected backend.
2. Keep diagnostics source-aware and reject unsupported or ill-typed syntax before code generation.
3. Update focused language docs and examples when changing user-visible syntax or component behavior.
4. Run the narrowest relevant compiler/build checks. Inspect generated Swift/Kotlin for platform correctness; a successful Rust build alone does not prove generated native code compiles in a host app.
5. Avoid adding dependencies or runtime layers unless the feature requires them and their cost is clear.
