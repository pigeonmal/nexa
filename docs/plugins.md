# Nexa plugins

Nexa plugins are compile-time packages. A package may contain pure Nexa source,
native Swift/Kotlin contracts, platform implementation files, and assets. The
compiler resolves the package from the app's `plugin "path" as Namespace`
declaration; generated code uses direct native calls and does not include a
runtime registry, reflection, JSON bridge, or RPC layer.

## Package manifest

Every package starts with `plugin.config.nx`:

```text
plugin {
    schema: 2
    id: "dev.example.video"
    version: "1.0.0"
    sources {
        nexa: "plugin.nx"       // optional
        native: "native.nxid"   // optional
    }
    ios {
        minVersion: "17.0"
        sources: ["ios/Sources/**"]
    }
    android {
        minSdk: 26
        sources: ["android/src/main/kotlin/**"]
    }
    assets: ["assets/**"]
}
```

The manifest is the package source of truth. Paths are relative to the package
root and cannot escape it. A package must declare at least Nexa source, a native
contract, or assets. Swift Package Manager/Maven dependency declarations and
custom source glob copying are planned next; the current project generator
uses the conventional platform source roots shown above.

Create a package with:

```sh
nexa plugin init dev.example.video --out VideoPlugin --name VideoPlayer
```

The command is deterministic and preserves existing files. `--kind pure`
creates `plugin.nx` and `assets/`; the default native package creates
`native.nxid`, Swift/Kotlin source roots, and `plugin.config.nx`.

Validate a package and generate direct bindings with:

```sh
nexa plugin check VideoPlugin
nexa plugin generate VideoPlugin --target swift
nexa plugin generate VideoPlugin --target kotlin --package dev.example.video
```

The C header generator and manually authored `abi.nxabi` contract were removed
from the normal plugin workflow. Swift/Kotlin plugins already compile against
their native language contracts directly; an optional C++ adapter will be
introduced later with generated ownership and exception handling.

## Pure Nexa package

A pure package has a manifest with a `sources.nexa` entry:

```text
plugin {
    schema: 2
    id: "dev.example.design"
    version: "1.0.0"
    sources { nexa: "plugin.nx" }
    assets: ["assets/**"]
}
```

Declare it from an app:

```nexa
plugin "../DesignPlugin" as Design

app Example {
    body {
        DesignLabel(text: "Reusable")
    }
}
```

`plugin.nx` is loaded through the normal source graph. Unreachable components
are removed by IR reachability analysis, so a pure plugin adds no registry or
native dependency. Reachable assets are copied to Android `drawable-nodpi` and
to an iOS asset catalog with deterministic names.

## Native contract (`native.nxid`)

The contract is separate from the `.nx` application grammar. It supports typed
models and platform contracts:

```text
struct PlayerOptions {
    quality: Float64
    saveToGallery: Bool = false
}

enum PlayerState { idle, ready, ended }
error PlayerError { invalidUrl, decodingFailed }

native class VideoPlayer {
    init(options: PlayerOptions)
    readonly property state: PlayerState
    property volume: Float64

    async fn prepare(url: String) throws PlayerError
    fn play()
    fn pause()
    event ended()
}

native component VideoView {
    prop player: VideoPlayer
    prop controls: Bool
    event tapped()
}
```

The parser validates duplicate names, type references, generic arity,
constructor/property/event parameters, and asynchronous throwing methods.
Generated Swift output contains value types and protocols such as
`VideoPlayerSpec`; generated Kotlin output contains equivalent data classes,
enums, and interfaces. A native class also gets a type alias named after the
Nexa class so component properties remain typed as `VideoPlayer`.

`service` declares stateless APIs. `interface` remains available for a shared
contract. `native class` represents an independently constructible stateful
object, and `native component` represents a platform visual contract. Native
object lifetime, events, platform implementations, and generated factories are
being implemented in the next plugin phases; the current compiler still
accepts direct interface method calls only.

## Compile-time options

Native contracts may declare scalar build options:

```text
config {
    cacheSizeMiB: Int32 = 64
    enableLogging: Bool = false
}
```

App values remain in the generated `nexa.config.nx` file:

```text
config {
    permissions {}
    plugins {
        VideoPlugin {
            cacheSizeMiB: 64
            enableLogging: false
        }
    }
}
```

The CLI validates required values, defaults, optional values, unknown options,
and scalar literal types before native generation. Reachable plugins receive
direct generated Swift/Kotlin constants; no runtime option map is emitted.

## Current boundary and next phases

Local package discovery, pure-source loading, typed native parsing, direct
Swift/Kotlin contract generation, asset reachability, and compile-time options
are implemented. Package installation/version resolution, native implementation
conformance checks, stateful object lowering, typed error recovery in `.nx`,
instance-scoped events, native visual component lowering, SPM/Maven dependency
injection, and optional generated C++ adapters remain planned work. See
`plugin-plan.md` for the full migration and test matrix.
