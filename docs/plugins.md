# Nexa plugins

Nexa keeps optional integrations outside the core compiler and default native
component set. The current tool can create an isolated plugin scaffold:

```sh
cargo run -p nexa-cli -- plugin init com.example.camera \
    --out CameraPlugin --name Camera --version 0.1.0
```

The scaffold contains:

- `nexa.plugin.json` with a stable format version, package id, version, IDL
  path, and iOS/Android implementation source paths;
- `interfaces.nxid`, a small typed plugin IDL kept outside the application
  `.nx` grammar;
- `ios/Sources/Camera.swift` as the iOS implementation boundary;
- `android/src/main/kotlin/.../Camera.kt` as the Android implementation
  boundary;
- a README that records the package shape and current limitations.

The command is deterministic and idempotent: unchanged files are preserved and
existing edits are not overwritten when their contents differ.

## Typed IDL

Declare value models and native methods in `interfaces.nxid`:

```text
type CameraOptions
type Photo
type CameraError: Error

interface Camera {
    async fn takePhoto(options: CameraOptions) -> Result<Photo, CameraError>
}
```

The parser checks identifiers, duplicate names, parameter types, generic type
syntax, and the rule that `Result<Success, Failure>` methods are asynchronous.
Use `nexa plugin check <plugin-directory|interfaces.nxid>` to validate it with
source locations.

`nexa plugin generate <plugin-directory|interfaces.nxid> --target swift` emits
a direct Swift protocol and value/error model declarations. The Kotlin target
emits a direct interface and Kotlin model/error declarations; asynchronous
`Result<Success, Failure>` methods use native `async throws` on Swift and
`suspend` on Kotlin. Pass `--package com.example.plugin` for a Kotlin package
other than the default generated package. Primitive and collection types map directly to native
types, including `Array`, `Set`, `Map`, `Pair`, and `Triple`.

This is the first typed binding boundary. It does not yet install dependencies,
generate `.nx` call sites, or connect the generated interface to the application
compiler. Those steps remain separate roadmap work so optional plugins cannot
add core runtime or dependency overhead to apps that do not use them.
