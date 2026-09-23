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
        frameworks: ["AVFoundation"]
        xcframeworks: ["ios/Frameworks/VideoSDK.xcframework"]
        privacyManifest: "ios/PrivacyInfo.xcprivacy"
        resources: ["ios/Resources/model.dat"]
    }
    android {
        minSdk: 26
        sources: ["android/src/main/kotlin/**"]
        aars: ["android/libs/video-sdk.aar"]
        resources: ["android/resources/model.dat"]
        proguardRules: ["android/rules/video-sdk.pro"]
    }
    cpp {
        standard: "c++20"
        sources: ["cpp/Sources/**"]
        headers: ["cpp/include/**"]
    }
    assets: ["assets/**"]
}
```

The manifest is the package source of truth. Paths are relative to the package
root and cannot escape it. A package must declare at least Nexa source, a native
contract, or assets. The `ios.sources` and `android.sources` arrays are resolved
during project generation and accept files, directories, or `*`/`?`/`**` path
patterns. When omitted, the generator falls back to the conventional platform
source roots. Platform minimums and native dependencies are declared here too.
The optional `cpp.standard` field accepts `c++17`, `c++20`, or `c++23` as the
minimum ISO C++ level needed by the implementation. Generated iOS and Android
targets use the highest minimum requested by their reachable C++ plugins;
when none is declared, generated C++ continues to use C++20.

```text
plugin {
    schema: 2
    id: "dev.example.video"
    version: "1.0.0"
    sources { native: "native.nxid" }
    ios {
        minVersion: "17.0"
        sources: ["ios/Sources/**"]
        dependencies {
            swiftPackage {
                url: "https://github.com/vendor/player-sdk.git"
                from: "2.0.0"
                products: ["PlayerSDK"]
            }
        }
        usageDescriptions {
            NSCameraUsageDescription: "Scan documents using the camera."
        }
        entitlements {
            "aps-environment": "development"
            "com.apple.developer.associated-domains": ["applinks:example.com"]
            "com.apple.developer.networking.wifi-info": true
        }
        linkerFlags: ["-ObjC"]
    }
    android {
        minSdk: 26
        sources: ["android/src/main/kotlin/**"]
        repositories: ["https://maven.example.com/releases"]
        dependencies: ["com.vendor:player-sdk:2.0.0"]
        permissions: ["android.permission.INTERNET"]
    }
}
```

`nexa generate` adds Swift packages, products, and system frameworks to the
Xcode target; it adds Maven coordinates and HTTPS Maven repositories to the
Gradle app, and raises deployment minimums to satisfy reachable plugins.
Repeated dependencies and repositories are deduplicated; incompatible
versions or ambiguous Swift product names fail generation. Generated Android
projects enable Gradle dependency locking. Resolve dependencies with
`gradle --write-locks :app:assembleRelease` from the generated `android/`
directory and keep the resulting lockfile with the project. Xcode writes its
SwiftPM `Package.resolved` after resolving packages; keep that file with the
project for repeatable builds.
Local XCFrameworks are copied into the iOS project, linked, and embedded with
code signing. AARs are copied into `android/app/libs` and added as file
dependencies. Platform resource paths are copied into the iOS app's
`NexaPluginResources` folder or Android's `assets/nexa/plugins/PluginN` tree.
Each declared iOS privacy manifest is retained as
`NexaPluginN.bundle/PrivacyInfo.xcprivacy` inside the app resources. Android
ProGuard rules are appended to the generated `proguard-rules.pro`; rule
contents remain plugin-owned and are checked by R8 during a Gradle build.
Declared artifact and resource contents participate in project cache keys.
Android permissions are merged into the generated manifest. iOS purpose strings
are merged into `Info.plist`; conflicting text for the same key is an error.
Plugin entitlements accept strings, booleans, and arrays of strings. Nexa merges
them into `Nexa.entitlements`, configures `CODE_SIGN_ENTITLEMENTS`, and rejects
conflicting values for the same key. Apple may require matching app identifiers
or provisioning-profile capabilities for an entitlement when signing a device
build.
The `ios.linkerFlags` array passes each entry as one escaped `OTHER_LDFLAGS`
argument and preserves declaration order across reachable plugins.
These declarations are retained only for plugins that survive compiler
reachability pruning.

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
nexa plugin generate VideoPlugin --target cpp
```

`nexa plugin check` parses the manifest and native IDL, syntax-checks the
declared Nexa source and its relative import graph, and verifies that declared
platform source patterns, assets, local native binaries, platform resources,
privacy manifests, and Android shrinker rules resolve to files inside the
package. When Xcode and the iOS simulator SDK are available and the plugin has no SwiftPM
dependencies, it also type-checks the Swift sources against the generated
contract. SwiftPM dependencies are resolved by the generated Xcode project.
When `kotlinc` is installed, it type-checks
dependency-free Kotlin services and classes against the generated contract.
Implementations that import external Android APIs, declare Maven dependencies,
or expose Compose components are checked by the generated Gradle project.

The C header generator and manually authored `abi.nxabi` contract were removed
from the normal plugin workflow. Swift/Kotlin plugins compile against their
native language contracts directly. `nexa plugin generate --target cpp` emits
an opt-in C++ implementation contract and is compiler-checked. iOS project
generation directly adapts synchronous, non-throwing scalar, `String`, and
`Bytes` service and native-class members through Swift/C++ interoperability,
including optional primitive, `String`, and `Bytes` values and owned
native-class references. Optional inputs use generated C++ factories and
optional results convert explicitly into Swift `Optional`. The generated Swift
adapters also convert flat primitive, string, and byte `Array` values through
generated C++ `std::vector` aliases. They explicitly convert between Swift
`String` and C++ `std::string`, and
between Swift `Data` and `std::vector<uint8_t>`. iOS C++ adapters bridge
`Set<Bool>`, integer sets, and `Set<Bytes>` through generated vector facades
around the C++ `std::set` contract. They reject floating-point sets (NaN
ordering) and string sets (Swift's canonical-equivalence equality differs from
bytewise C++ ordering). iOS map adapters also accept supported scalar keys with
primitive, string, byte, compatible-set, or nested-array values through
generated entry vectors; generated Swift/C++ typechecks cover numeric, string,
byte, and set values in maps.
Android project generation emits
Kotlin service/class adapters and JNI for synchronous, non-throwing `Bool`,
signed and unsigned integer, floating-point values, plus `String` and `Bytes`.
Unsigned Kotlin values use signed primitive carriers at the JNI boundary while
preserving their full bit patterns. JNI strings
are converted between UTF-16 and UTF-8 with length-aware handling, including
embedded NUL and supplementary Unicode characters. Byte arrays use Kotlin
`ByteArray` and C++ `std::vector<uint8_t>` with length-aware copies. Native
Android classes must declare a parameterless `dispose()` so
the generated adapter can release each C++ instance deterministically. Android
adapters bridge flat primitive `Array` values through Kotlin/JNI primitive
arrays and C++ `std::vector`; unsigned values preserve their bits through signed
JNI carriers. `Array<String>` and `Array<Bytes>` use JNI object arrays with
length-aware conversion and local-reference cleanup. `Set` supports Boolean,
integer, and String elements through JNI array carriers and C++ `std::set`.
Floating-point and byte-array sets remain unsupported because their Kotlin and
C++ equality/order semantics differ. Android `Map` keys support primitive or
String types; map values support primitive, string, or byte values, arrays of
primitive/string/byte values, compatible primitive or string sets, and
recursively nested maps. Generated JNI adapters convert these shapes with
headless C++ compilation and optional host-JVM round-trip coverage.
Floating-point keys, byte-array sets, optional maps, and collection shapes
outside that subset remain unsupported to preserve Kotlin/C++ map semantics.
C++ event, async, and throwing adapters also remain unsupported.
Android optional C++ values support `Bool`, signed and unsigned integers,
floating-point, `String`, and `Bytes`. Generated R8 rules
preserve JNI lookup names. Plugin authors implement the generated C++ contract
and do not write JNI. On a target using C++ adapters, platform source files may
provide UI/native-component implementations, but should not also define the
generated service plugin object or `<NativeClass>Impl` types.


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
error PlayerError {
    invalidUrl
    decodingFailed(message: String)
}

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
    content
    prop player: VideoPlayer
    prop controls: Bool = true
    event tapped()
    event progressChanged(position: Float64, duration: Float64)
}
```

The `content` marker declares one required child-content slot. A call must then
provide a Nexa child block, which is passed directly to the generated SwiftUI
and Compose wrappers. Without the marker, child blocks are rejected. Native
component properties can declare scalar or nullable literal defaults; Nexa
inserts omitted defaults during compilation.

The parser validates duplicate names, every field and argument type reference,
generic arity, scalar default values, recursive inline value layouts,
constructor/property/event and error-variant payloads, and asynchronous
throwing methods. `throws` and the failure arm of `Result<Success, Failure>`
must name a declared `error` type. Generated Swift errors are associated-value
enums conforming to `Error`; Kotlin errors are sealed `Exception` classes with
object/data-class variants. Kotlin throwing methods carry
`@Throws(ErrorType::class)` metadata. Generated Swift output contains value
types and protocols such as
`VideoPlayerSpec`; generated Kotlin output contains equivalent data classes,
enums, and interfaces. A native class contract expects the plugin implementation
to provide a concrete `{Name}Impl` class; generated bindings alias `{Name}` to
that implementation so construction stays a direct native call. `nexa generate`
copies those generated contracts into the host project automatically: Swift
bindings are added as independent Xcode source inputs, while Kotlin bindings
are emitted in the implementation package and imported by generated app code.
Each generated native-class binding also has a private conformance-check
function, so the host compiler checks that `{Name}Impl` satisfies its declared
contract without routing app calls through a wrapper. Swift native-class
contracts are `@MainActor` isolated. Kotlin plugin calls use the caller's
coroutine dispatcher; Nexa does not add implicit background dispatch.
An error payload named `message` overrides Kotlin's inherited
`Throwable.message` property, preserving the IDL field while keeping the
generated exception valid Kotlin.

Throwing async APIs must be called inside an explicit recovery block:

```nexa
OnAppear async {
        try {
            await player.prepare(url: source)
        } catch {
            case Video.PlayerError.invalidUrl {
                loadFailed = true
            }
            case Video.PlayerError.decodingFailed(message) {
                loadError = message
            }
        }
}
```

Swift emits typed `throws(PlayerError)` contracts and `do`/`catch`; Kotlin emits
`@Throws` contracts, sealed error classes, and `try`/`catch`. Case arms bind
declared payloads. Without `else`, the cases must cover every variant of the one
declared error type thrown in that block; add `else { ... }` for untyped errors,
multiple error types, or fallback handling.
Native component contracts also generate a direct SwiftUI wrapper or Compose
function that calls `{Name}Impl` in the plugin source. Use the qualified Nexa
syntax so the export is unambiguous:

```nexa
app VideoDemo {
    let player = VideoPlayer()
    state tapped = false

    body {
        VideoPlayer.VideoView(player: player, controls: true).onTapped {
            tapped = true
        }
    }
}
```

Native components accept IDL `prop` values and forward them directly to the
platform implementation. Their events attach to the component call with a
dot modifier. Native-class events use an action statement on the native object:

```nexa
OnAppear {
    player.ended { playbackFinished = true }
    player.progressChanged { position, duration ->
        progress = position / duration
    }
}

VideoPlayer.VideoView(player: player, controls: true).onProgressChanged {
    position, duration ->
    progress = position / duration
}
```

The compiler checks the event name, payload arity, and payload types. It lowers
each subscription to assignment of that instance's generated `on<Event>`
callback property, so two players keep independent callbacks and no global
event bus is introduced. A callback property holds one handler; assigning the
same event again replaces its previous handler. Payload names must not shadow
existing values. Native-component handlers are direct closure arguments to the
generated wrapper, without a registry or event bus. Each call owns its callback
value, so separate component instances can update separate state. Passing a
child block is accepted only when the IDL declares a `content` slot; that block
is lowered and passed to the platform implementation.

`service` declares stateless APIs. `interface` remains available for a shared
contract. `native class` represents an independently constructible stateful
object, and `native component` represents a platform visual contract. Native
class constructors and instance method calls are lowered as direct object
calls, so two constructor expressions produce two independent native objects.
Read-only native properties are lowered as direct member access. Methods accept
named arguments, and `Void` methods can be invoked in event actions, including
`await` for asynchronous methods. Mutable native properties can be assigned in
actions (for example, `player.volume = 0.5`); semantic analysis rejects writes
to readonly properties. Disposal declarations on native classes must be
synchronous, non-throwing, parameterless `Void` methods. The compiler rejects
duplicate disposal and uses that may occur after disposal within an action
sequence, including conditional and loop paths. A mutable native-class binding
can be reset with a fresh constructor only after disposal when no immutable
alias still refers to that instance. Direct aliases share the same disposal
identity within an action sequence. Cross-callback analysis rejects a
disposal from one UI/lifecycle callback when another callback can use or
dispose that same declared instance. It follows app and screen state bindings
across those scopes, plus component-local state and parameters within one
component. App and screen ownership scope `OnDisappear` cleanup: a screen cannot
dispose an app-owned object that may still be used by another route. Compose
scopes screen state to each back-stack entry, and SwiftUI destinations use a
unique route identity with route-owned state. Native route parameters remain
scalar, and resources reused after teardown are not modeled yet. Borrowed
component parameters keep the same identity through nested component calls and
cannot be disposed by the child. Implementations should clear callbacks they
retain.
Typed error variants and payload binding are supported in `.nx`;
throwing calls require an explicit `try`/`catch` recovery block.
Constructor parameter defaults in the IDL are also reserved; use an explicit
zero-argument constructor in the current slice when the Nexa call should be
`VideoPlayer()`.

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
Swift/Kotlin contract generation, asset reachability, compile-time options,
qualified native components, manifest-declared iOS purpose strings and Android
permissions, typed iOS signing entitlements, compile-time native-class
conformance checks, and scope-aware native-object `OnDisappear` checks are
implemented.
Native-class and native-component event subscriptions are direct and
instance-scoped. Native-class parameters on custom components are borrowed;
the component cannot dispose its caller's object, and the containing view keeps
the object live until disappearance. Package installation/version resolution,
complete runtime coverage for native object instances, and broader generated
C++ adapters remain planned work. See
[plugin-architecture.md](plugin-architecture.md)
for the current architecture report and migration map, and `plugin-plan.md` for
the full design and test matrix.
