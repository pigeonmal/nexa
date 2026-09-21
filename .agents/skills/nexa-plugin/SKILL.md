---
name: nexa-plugin
description: "Use when designing or implementing an optional Nexa plugin, platform integration, typed native binding, or plugin-system architecture for features such as storage, maps, or device capabilities."
---

# Nexa Plugin Development

Help extend Nexa with optional features that users can install without placing every integration in the core framework. Read `plan.md`'s plugin-system and platform-implementation sections and inspect the current repository before proposing interfaces. The plugin IDL, binding generator, package manager, and install workflow are roadmap items unless present in code; never imply they already work.

## Boundaries

- Keep the default Nexa compiler and core component set usable without optional plugins or their native dependencies.
- Give each plugin a clear public API, platform implementations, dependency declarations, and generated bindings where the current architecture supports them. Keep platform-specific code isolated by platform and plugin.
- Reuse the generated core network/path/file bindings for transport and storage-adjacent plugin work. Network plugins should call URLSession on iOS and Cronet on Android, preserve typed request options, and avoid introducing OkHttp, a second image loader, JSON/RPC, or a runtime service locator.
- Core value types available to future typed plugin APIs include scalars, `Array<T>`, `Set<T>`, `Map<K, V>`, `Pair<A, B>`, and `Triple<A, B, C>`. Set elements and map keys are currently limited to scalar `String`, `Bool`, or numeric types; collection lookup and mutation are not exposed by the core language yet. Do not claim that future plugin APIs or generic collection operations already exist.
- Prefer statically typed interfaces and generated direct calls. Avoid JSON/RPC, string-based dispatch, reflection, dynamic dictionaries, and per-frame boundary crossings.
- Do not put plugin-specific behavior in the parser, common UI nodes, or both backends unless it is truly part of the shared language contract.
- Do not implement SQLite, MMKV, maps, or another example integration merely because it appears in the roadmap. First establish the plugin-system capability or implement only the integration the user requested.
- Expose platform differences intentionally when APIs cannot share the same behavior; do not hide incompatible semantics behind a misleading common abstraction.

## Implementation workflow

1. Determine which pieces exist today: plugin metadata, IDL, dependency resolution, code generation, and Swift/Kotlin binding support.
2. State the smallest vertical slice needed, including how a `.nx` app declares and calls it and how each native backend resolves it.
3. Keep plugin API/schema, platform implementation, and compiler integration in separate maintainable modules.
4. Make generated native bindings typed and direct, with ownership and error behavior made explicit.
5. Document installation, platform setup, supported targets, and current limitations. Do not claim installability until the package/install path exists.
6. Validate the plugin independently and validate a minimal Nexa app that uses it; inspect output for both platforms that the plugin claims to support.
