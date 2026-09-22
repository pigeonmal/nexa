# Compiler diagnostics and optimization

Nexa performs diagnostics before native code generation. A successful compile can therefore report warnings while still producing the typed IR and native Swift/Kotlin source.

## Warnings

The compiler reports:

- unused app or component `state` declarations,
- unused immutable `let` constants,
- unused function-local `let` constants,
- unused function parameters,
- unused action-loop bindings (`for item`, `for (key, value)`),
- unused `FastList` row bindings (`index` and `item`),
- unused pure app functions,
- unused custom-component parameters,
- conditions that are provably always `true` or `false`.

Warnings include the source file, line, and column. Unused declarations are removed from the generated native source when the IR proves that no reachable node or action reads them. Component parameters remain in their declaration and call signatures so component APIs stay consistent; they are diagnosed but not removed automatically.

Use the normal commands to display warnings:

```sh
cargo run -p nexa-cli -- check app.nx
cargo run -p nexa-cli -- build app.nx --target swift
```

Use `--deny-warnings` in CI or release checks when warnings must fail the command:

```sh
cargo run -p nexa-cli -- check app.nx --deny-warnings
cargo run -p nexa-cli -- build app.nx --target kotlin --deny-warnings
```

Loop-binding diagnostics are lexical: a binding is considered used when it appears anywhere in that loop's condition, iterable, or nested action body. The compiler keeps the warning source span at the loop declaration so editors can point to the binding even though the parser currently stores one span for the complete loop statement.

This follows the same separation used by Rust lint levels: diagnostics are warnings by default and can be promoted to errors at the command boundary. See the [Rust lint levels](https://doc.rust-lang.org/rustc/lints/levels.html) reference for the model Nexa follows.

## Native-code optimization

The compiler runs a platform-independent IR pass after semantic lowering and before either backend. It currently:

- folds pure boolean, scalar-comparison, numeric-addition, and literal-membership expressions,
- removes statically unreachable UI and event branches,
- removes unused app and component state declarations,
- removes unused value-struct declarations while retaining transitively used struct dependencies,
- removes unused pure function-local constants while preserving locals whose initializers contain async work,
- removes pure functions that are unreachable from app state, UI, actions, and reachable components,
- preserves dynamic state and environment conditions,
- keeps Swift and Kotlin component mappings unchanged.

The pass does not add a runtime or bridge. It reduces generated source before the Swift and Kotlin compilers run, leaving platform-specific optimization to the native toolchains. This matches Apple’s guidance to measure changes and keep optimization close to the native compiler, and Kotlin’s guidance to rely on release compiler optimization and dead-code elimination rather than a custom runtime layer. See [Apple performance guidance](https://developer.apple.com/documentation/xcode/improving-your-app-s-performance/) and [Kotlin compiler options](https://kotlinlang.org/docs/compiler-reference.html).

## Platform-specific lowering

`platform ios { ... }` and `platform android { ... }` are resolved from the CLI build target. The compiler keeps only the selected block before semantic lowering, reachability analysis, and backend generation. `nexa check` compiles both target variants so both branches remain validated; a target build emits only its native branch.
