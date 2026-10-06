---
name: nexa-plugin
description: "Use when designing or implementing an optional Nexa plugin, platform integration, typed native binding, or plugin-system architecture for features such as storage, maps, or device capabilities."
---

# Nexa Plugin Development

Extend Nexa with an optional capability an app can adopt without the core framework depending on it.

## Rule 1: the contract is the source of truth

`plugins/<name>/native.nxid` defines structs, enums, typed error variants, services, interfaces,
native classes and components, properties, events, methods, compile-time options, and async
throwing boundaries. `plugins/<name>/plugin.config.nx` defines package identity, source globs,
platform minimums, native dependencies, iOS usage descriptions and entitlements, Android
permissions, and assets.

Read both before proposing an interface. Do not design against a remembered contract.

> [!IMPORTANT]
> `nexa-plugin-idl` **discards `//` comments** while lexing and records no source spans. Doc
> comments in a `.nxid` are invisible to the parser, and a generator cannot point at the line it
> rendered. This is why no `nexa plugin doc` command exists today. If you need generated `.nxid`
> documentation, that requires comment retention in the lexer first — check whether it has landed
> before planning around it.

## Workflow

```bash
nexa plugin init <plugin.id> --out <dir> --name <TypeName> [--kind native|pure]
nexa plugin check  <package-directory|native.nxid>
nexa plugin generate <package-directory|native.nxid> --target <swift|kotlin|cpp> [--package <name>] [--out <dir>]
```

`init` scaffolds `plugin.config.nx` plus `native.nxid` (or `plugin.nx` for `--kind pure`).
`check` validates the manifest and IDL, confirms declared source globs and asset roots exist,
syntax-checks any Nexa source, and type-checks Swift/Kotlin directly when the SDKs and no
external dependencies are involved. `generate` writes `<Namespace>.swift`, `<Namespace>.kt`, or a
typed C++ contract header.

1. Establish the smallest vertical slice: how an app declares the plugin, how it calls it, and how
   each backend resolves it.
2. Keep API/schema, platform implementation, and compiler integration in separate modules.
3. Keep generated bindings typed and direct, with ownership and error behavior explicit.
4. Validate the plugin alone, then a minimal app that uses it. Inspect output for every platform
   the manifest claims.

## Boundaries

- Core must stay usable with no plugins and none of their native dependencies.
- Reuse the generated core network/path/file bindings. Network plugins call URLSession on iOS and
  Cronet on Android, preserving typed request options. Do not add OkHttp, a second image loader,
  JSON/RPC, or a service locator.
- Prefer direct typed calls. Avoid reflection, string dispatch, dynamic dictionaries, and per-frame
  boundary crossings.
- Declare compile-time options in `native.nxid`; users set them under `plugins { Namespace { ... } }`
  in `nexa.config.nx`. Validate required/optional/defaulted/typed values before generation and emit
  direct native constants, never a runtime dictionary.
- Do not put plugin-specific behavior in the parser, common UI nodes, or both backends unless it is
  genuinely part of the shared language contract.
- Do not implement SQLite, MMKV, maps, or any other catalog integration just because it exists. Build
  what was asked, or establish the plugin-system capability it depends on.
- There is no package manager or install workflow. Do not describe a plugin as installable; a local
  `.nx` app declares it with `plugin "path" as Namespace`.

## Contract capabilities

- **Value types**: scalars, `Bytes`, `Array<T>`, `Set<T>`, `Map<K,V>`, `Pair<A,B>`, `Triple<A,B,C>`,
  app structs, and app enums. Set elements and map keys are limited to scalars. Optionals are not
  storable.
- **Generic methods** (`fn setObject<T>(...)`) work on Swift and Kotlin. Each call site binds `T`;
  the compiler emits a concrete reader or writer per bound type. Sets and maps serialize in a
  canonical byte order so values are portable across platforms; that layout is fixed, so a struct
  change needs a new key or a version field. On Android the codec lives in the generated
  `dev.nexa.core` package, because the plugin itself is generated into its own package.
- **Qualified visual components** (`Namespace.Component(prop: value)`) lower to a direct
  SwiftUI/Compose wrapper. Declared content slots and event handlers are supported.
- **Async throwing calls** require explicit `try { ... } catch { ... }`. Catch cases must match
  declared error variants; an `else` arm handles untyped failures.
- **Errors**: Swift uses associated-value `Error` enums; Kotlin uses sealed `Exception` subclasses
  with `@Throws`.
- **Ownership**: generated Swift native-class contracts are `@MainActor` isolated; Kotlin native
  methods stay on the caller's dispatcher. Nexa never silently moves plugin work to a background
  dispatcher — choose dispatch explicitly.

## Manifest notes

- `schema` must be `2`.
- `ios.linkerFlags` is a list of individual arguments; Nexa preserves order and rejects empty
  arguments and control characters.
- A Swift package declares exactly one of `from`, `branch`, or `revision`. Prefer `revision` when
  upstream has no SwiftPM release tags, since a range can resolve to a commit with no
  `Package.swift`.
- Android Maven coordinates live under `android.dependencies`; they are not app-level registry
  identifiers.
- Generated C++ adapters support a narrower signature set than Swift/Kotlin: synchronous and
  non-throwing async scalars, strings, bytes, selected collections, and async typed errors with
  primitive/string/byte payloads. Do not assume parity.
- Plugin-owned assets go under `assets/` and are copied into Android `drawable-nodpi` and an iOS
  asset catalog with deterministic, lowercase-safe names.

## Platform requirements

Read the published tables rather than recalling floors:

- [`README.md#official-native-plugins`](../../../README.md) — every package with its iOS and Android minimums
- [`docs/plugins.md#deployment-targets-and-permissions`](../../../docs/plugins.md) — permissions and usage strings
- [`docs/plugins.md#entitlements-background-modes-and-services`](../../../docs/plugins.md) — entitlements, background modes,
  app delegate, Android services and metadata
- [`docs/plugins.md#native-dependency-coordinates`](../../../docs/plugins.md) — SwiftPM and Maven coordinates

All four are generated from the manifests and gated by `cargo test -p nexa-cli --test
plugin_reference`. After editing any `plugin.config.nx`, run
`NEXA_UPDATE_SNAPSHOTS=1 cargo test -p nexa-cli --test plugin_reference` and commit the result.

## Worked example

[`examples/plugins/fast-math/`](../../../examples/plugins) is a complete native package: manifest,
`.nxid` contract, and C++ implementation. Read it before writing your first one.

## Verification

```bash
export CARGO_TARGET_DIR=/tmp/nexa-verification-target
nexa plugin check plugins/<name>
cargo check --workspace --all-targets && cargo clippy --workspace --all-targets
```
