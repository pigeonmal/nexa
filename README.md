# Nexa

Nexa is an early ahead-of-time compiler prototype for a shared mobile language that emits native SwiftUI and Jetpack Compose source. Its frontend and typed intermediate representation are Rust; generated applications use the platform UI frameworks directly and do not include a JavaScript or Dart runtime.

The current implementation covers the compiler foundation and a growing native-control slice. It parses app state, checks primitive and array types and bindings, lowers to a platform-independent IR, and emits native SwiftUI or Compose controls, including virtualized range and collection lists. It is not yet a complete mobile framework or a project generator.

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
cargo run -p nexa-cli -- check examples/themed-app.nx
cargo run -p nexa-cli -- check examples/custom-components.nx
cargo run -p nexa-cli -- check examples/conditional-logic.nx
cargo run -p nexa-cli -- build examples/themed-app.nx --target swift --out /tmp/ThemedApp.swift
cargo run -p nexa-cli -- build examples/themed-app.nx --target kotlin --out /tmp/ThemedApp.kt
cargo run -p nexa-cli -- build examples/custom-components.nx --target swift --out /tmp/CustomComponents.swift
cargo run -p nexa-cli -- build examples/custom-components.nx --target kotlin --out /tmp/CustomComponents.kt
cargo run -p nexa-cli -- build examples/conditional-logic.nx --target swift --out /tmp/ConditionalLogic.swift
cargo run -p nexa-cli -- build examples/conditional-logic.nx --target kotlin --out /tmp/ConditionalLogic.kt
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

Compiler orchestration and semantic analysis are separate modules inside `nexa-compiler`. Native generation lives in backend-local `src/generator/` modules, with component-specific files for controls, inputs, images, layout, navigation, lists, keyboard behavior, expressions, state, and formatting. A shared IR walker keeps structural scans consistent across backends; each backend analyzes a module once and uses that result to emit only the imports and native helpers the generated app needs. This keeps platform concerns out of the shared IR and makes compiler changes easier to review and maintain.

User-defined `.nx` components can live in imported files, declare typed inputs and private state, compose other components, and compile directly into native SwiftUI or Compose declarations. Each reachable component has one generated native declaration, and every use calls that declaration directly; imports resolve relative to the source file and are statically compiled, while unreachable component declarations are omitted from generated output.

## Project skills

The repository includes three project-scoped Codex skills under `.agents/skills/`:

- `nexa-framework` for the compiler, IR, core components, native backends, and performance architecture.
- `nexa-plugin` for designing optional, typed platform integrations.
- `nexa-app` for helping app authors build with the supported `.nx` language without writing Swift or Kotlin.

These skills distinguish the current prototype from the longer-term goals in `plan.md` and guide changes toward modular, native code generation.

## Language slice

See the [language guide](docs/language.md) for syntax, supported types, themes, current limits, and native mappings, and the [language design decisions](docs/language-design.md) for how Nexa adopts or defers Kotlin/Swift concepts. The full roadmap remains in [plan.md](plan.md).
