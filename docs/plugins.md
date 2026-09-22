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

## Use a local plugin from `.nx`

An application can declare a local plugin directory (or its `interfaces.nxid`
file) at the top level of the entry file:

```nexa
plugin "../CameraPlugin" as Camera

app CameraPreview {
    state status: String = "idle"

    body {
        Text(status)
        OnAppear async {
            status = await Camera.ping(value: "ready")
        }
    }
}
```

The compiler resolves the path relative to the entry `.nx` file, validates the
IDL, and adds the declared interface methods to the same typed call checker as
app functions. Calls use the namespace from the declaration, require `await`
for asynchronous methods, and lower directly to the platform implementation:
`CameraPlugin.shared.method(...)` on iOS and `CameraPlugin.instance.method(...)`
on Android. There is no runtime registry, reflection, JSON/RPC layer, or boxed
plugin call object.

`nexa generate` also copies plugin implementation sources from
`ios/Sources/**/*.swift` and `android/src/main/kotlin/**/*.kt` into the generated
native project. The plugin author supplies the methods declared by the IDL in
those sources. The current project integration is local and source based: it
does not resolve versions, download packages, or add third-party dependencies.

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

This is the first typed binding boundary. IDL checking and direct local `.nx`
calls are integrated without adding optional dependencies to applications that
do not declare a plugin. The generated native bindings support named models;
the current `.nx` call surface is limited to scalar, optional, collection,
pair, and triple values, so native model construction remains in the plugin
implementation. Package installation, version resolution, generated
implementation methods, typed error values in `.nx`, and Rust/C bindings remain
roadmap work.
