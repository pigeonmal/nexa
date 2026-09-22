---
name: nexa-plugin
description: "Use when designing or implementing an optional Nexa plugin, platform integration, typed native binding, or plugin-system architecture for features such as storage, maps, or device capabilities."
---

# Nexa Plugin Development

Help extend Nexa with optional features that users can install without placing every integration in the core framework. Read `plan.md`'s plugin-system and platform-implementation sections and inspect the current repository before proposing interfaces. The package manager and install workflow are roadmap items unless present in code; never imply they already work.

## Boundaries

- Keep the default Nexa compiler and core component set usable without optional plugins or their native dependencies.
- Give each plugin a clear public API, platform implementations, dependency declarations, and generated bindings where the current architecture supports them. Keep platform-specific code isolated by platform and plugin.
- Reuse the generated core network/path/file bindings for transport and storage-adjacent plugin work. Network plugins should call URLSession on iOS and Cronet on Android, preserve typed request options, and avoid introducing OkHttp, a second image loader, JSON/RPC, or a runtime service locator.
- Core value types available to typed plugin APIs include scalars, `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>`. Set elements and map keys are limited to scalar `String`, `Bool`, or numeric types. Arrays, sets, and maps support compile-time checked lookup, membership, direct action mutation, and native iteration; inline `map`/`filter`/`reduce` closures are available for `Array<T>`. Plugin IDL methods should still keep collection shapes explicit and avoid claiming generic or reference-type behavior that the core language does not support.
- Prefer statically typed interfaces and generated direct calls. Avoid JSON/RPC, string-based dispatch, reflection, dynamic dictionaries, and per-frame boundary crossings.
- Declare scalar compile-time options in `interfaces.nxid`; users set them under `plugins { Namespace { ... } }` in the generated `nexa.config.nx`. Validate required, optional, defaulted, and typed values before generation and expose them as direct native constants rather than a runtime dictionary.
- Do not put plugin-specific behavior in the parser, common UI nodes, or both backends unless it is truly part of the shared language contract.
- Do not implement SQLite, MMKV, maps, or another example integration merely because it appears in the roadmap. First establish the plugin-system capability or implement only the integration the user requested.
- The supported workflow is `nexa plugin init <plugin.id> --out <directory> --name <TypeName>`, followed by `nexa plugin check <plugin-directory|interfaces.nxid>` and `nexa plugin generate <plugin-directory|interfaces.nxid> --target <swift|kotlin|c>`. Pass `--package` for the generated Kotlin package. The scaffold includes `interfaces.nxid`; its parser validates typed models, methods, reserved-name collisions, declared type references, generic arity, compile-time config options, and asynchronous `Result<Success, Failure>` boundaries. Binding generation emits direct Swift protocols, Kotlin interfaces, or the validated C header subset and native model declarations. A local `.nx` app can declare `plugin "path" as Namespace` and call `await Namespace.method(...)`; the compiler resolves the IDL statically and generated projects include the plugin's conventional iOS/Android source trees. Dependency resolution, package installation, generated implementation methods, and typed error recovery remain future work.
- Use `nexa plugin init <plugin.id> --kind pure` for a source-only package. It creates `plugin.nx` and `assets/`; the app declares the package with the same `plugin "path" as Namespace` form, and the compiler loads reusable components/functions through the normal `.nx` graph. Use the default `--kind native` for IDL-backed Swift/Kotlin integrations. Pure plugins do not create native dependencies or bindings.
- Put plugin-owned images and other platform resources under `assets/`. Project generation copies reachable plugin assets into Android `drawable-nodpi` and an iOS asset catalog, with deterministic names. Keep resource names stable and lowercase-safe because generated Android lookup uses resource identifiers.
- Native scaffolds also contain `abi.nxabi`. Validate it with `nexa plugin check`; it is the versioned C ABI ownership/lifetime contract for future C/C++/Rust adapters. Do not describe a plugin as zero-copy unless it satisfies the call-scoped borrowed-buffer rules in that contract.
- Treat IDL bindings as direct typed calls with explicit ownership. The current native slice does not claim a C++/Rust zero-copy ABI; adding one requires a versioned buffer ownership/layout contract and generated adapters rather than reflection, JSON/RPC, or a runtime registry.
- Expose platform differences intentionally when APIs cannot share the same behavior; do not hide incompatible semantics behind a misleading common abstraction.

## Implementation workflow

1. Determine which pieces exist today: plugin metadata, IDL, dependency resolution, code generation, and Swift/Kotlin binding support.
2. State the smallest vertical slice needed, including how a `.nx` app declares and calls it and how each native backend resolves it. Prefer the existing direct local-plugin path over a registry or runtime lookup.
3. Keep plugin API/schema, platform implementation, and compiler integration in separate maintainable modules.
4. Make generated native bindings typed and direct, with ownership and error behavior made explicit.
5. Document installation, platform setup, supported targets, and current limitations. Do not claim installability until the package/install path exists.
6. Validate the plugin independently and validate a minimal Nexa app that uses it; inspect output for both platforms that the plugin claims to support. Keep implementation sources under `ios/Sources` and `android/src/main/kotlin` so project generation can include them deterministically.
