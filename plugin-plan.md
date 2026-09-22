# Task: Redesign and Implement Nexa Plugins v2

You are working on **Nexa**, an ahead-of-time compiler that compiles Nexa applications into fully native applications:

* iOS → Swift + SwiftUI
* Android → Kotlin + Jetpack Compose

Nexa is **not** a runtime bridge framework like React Native.

Nexa's main advantage is that it can resolve, validate, optimize, and generate everything at compile time.

I want you to perform a **full architectural review and redesign of the current Nexa plugin system**, then progressively implement the new architecture.

The priorities are:

1. Excellent developer experience for community plugin authors.
2. Strong compile-time type safety.
3. Maximum AOT optimization.
4. Minimal runtime overhead.
5. Native Swift/Kotlin integration without unnecessary intermediate layers.
6. Support for stateful native objects with multiple independent instances.
7. Support for native visual components.
8. Compatibility with real-world native SDKs.
9. Clean long-term extensibility.
10. Simple and understandable plugin architecture.

Do not blindly preserve the current architecture if a cleaner design is possible.

However, understand the current implementation thoroughly before making changes.

---

# 1. First: audit the current plugin implementation

Before modifying anything, inspect the complete existing implementation.

Find and document:

* plugin discovery;
* plugin parsing;
* `nexa.plugin.json`;
* `plugin.nx`;
* `interfaces.nxid`;
* `abi.nxabi`;
* Swift generation;
* Kotlin generation;
* C generation;
* Rust/C/C++ related code;
* native plugin validation;
* pure plugin behavior;
* native plugin behavior;
* namespace/import resolution;
* plugin pruning;
* cache fingerprints;
* build integration;
* Xcode project generation;
* Gradle project generation;
* asset handling;
* platform-specific source discovery;
* plugin configuration through `nexa.config.nx`;
* test coverage;
* error reporting.

Also determine exactly how this currently works:

```text
plugin "..." as Namespace
```

and how:

```text
Namespace.someMethod()
```

becomes:

```text
NamespacePlugin.shared
```

on iOS and:

```text
NamespacePlugin.instance
```

on Android.

Identify all assumptions that currently make a native plugin effectively a singleton.

Do not start large refactors until this architecture is understood.

---

# 2. Fundamental architecture rule

Nexa is an AOT native compiler.

The architecture must take advantage of that.

The normal execution path should be:

```text
Nexa
  ↓
generated Swift
  ↓
native Swift implementation
```

on iOS and:

```text
Nexa
  ↓
generated Kotlin
  ↓
native Kotlin implementation
```

on Android.

There should NOT be an unnecessary universal C ABI between Nexa and Swift/Kotlin.

Avoid:

```text
Nexa
  ↓
C ABI
  ↓
Swift/Kotlin
```

when Nexa is already generating Swift and Kotlin directly.

Likewise, avoid:

* runtime plugin registries;
* string-based module lookup;
* reflection;
* JSON serialization between Nexa and native code;
* dynamic dispatch when static resolution is possible;
* unnecessary wrappers;
* unnecessary allocations;
* runtime schema discovery.

Prefer compile-time generated direct calls.

---

# 3. Replace `nexa.plugin.json`

The current JSON manifest should be replaced with:

```text
plugin.config.nx
```

All Nexa configuration files should use Nexa-native configuration syntax consistently.

`plugin.config.nx` must become the **single source of truth** for plugin metadata.

The compiler should no longer create a JSON manifest and then independently rediscover the package structure through conventions.

A plugin package should conceptually look like:

```text
MyPlugin/
├── plugin.config.nx
├── plugin.nx                 // optional
├── native.nxid               // optional
├── ios/
│   └── Sources/
├── android/
│   └── src/main/kotlin/
├── cpp/                      // optional
└── assets/                   // optional
```

A plugin may contain:

* only Nexa code;
* only native code;
* both Nexa and native code;
* native visual components;
* optional C++ shared implementation.

Do NOT force a plugin to be exclusively:

```text
kind: pure
```

or:

```text
kind: native
```

That distinction is unnecessarily restrictive.

Instead determine plugin capabilities from what it declares.

Example conceptual configuration:

```text
plugin {
    schema: 2

    id: "dev.nexa.video-player"
    version: "1.0.0"

    nexa: ">=0.8 <1.0"

    sources {
        nexa: "plugin.nx"
        native: "native.nxid"
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

The exact syntax may be adapted to the existing Nexa config grammar, but preserve the architecture.

---

# 4. Replace `interfaces.nxid` with `native.nxid`

Rename:

```text
interfaces.nxid
```

to:

```text
native.nxid
```

The reason is that this file describes the native contract of a plugin.

It should not be required for a pure Nexa plugin.

The `.nxid` file must become a real strongly typed IDL instead of only listing method signatures plus opaque type names.

---

# 5. Remove ambiguous named types

The current system can apparently declare things such as:

```text
type CameraOptions
```

without describing what `CameraOptions` actually contains.

Remove or strongly restrict this behavior.

The compiler must understand the complete layout of data crossing the Nexa/native boundary.

Add proper declarations such as:

```text
struct CameraOptions {
    quality: Float64
    flash: FlashMode
    saveToGallery: Bool = false
}

enum FlashMode {
    off
    on
    auto
}
```

From this Nexa should be able to generate equivalent Swift and Kotlin types.

Example Swift:

```swift
struct CameraOptions {
    let quality: Double
    let flash: FlashMode
    let saveToGallery: Bool
}
```

Example Kotlin:

```kotlin
data class CameraOptions(
    val quality: Double,
    val flash: FlashMode,
    val saveToGallery: Boolean
)
```

Support at least:

* primitive types;
* strings;
* bytes;
* arrays/lists;
* optional values;
* structs;
* enums;
* typed errors;
* object references;
* callbacks/events where appropriate.

Design the IDL so it can grow later without ambiguity.

---

# 6. Introduce different native API concepts

Do not represent every native API with the same `interface` concept.

Introduce separate semantic constructs.

At minimum design:

```text
service
native class
native component
struct
enum
interface
error
```

Each has a different purpose.

---

# 7. `service`: stateless/global APIs

A `service` represents APIs where a global native instance makes sense.

Example:

```text
service Clipboard {
    fn copy(text: String)
    fn read() -> String?
}
```

It is acceptable internally for this to map to something like:

```swift
ClipboardPlugin.shared
```

and:

```kotlin
ClipboardPlugin.instance
```

because the API itself is logically global.

Examples:

* clipboard;
* vibration;
* device information;
* possibly notifications configuration;
* other stateless/system-level APIs.

Do not force every plugin to use this pattern.

---

# 8. `native class`: stateful native objects

This is one of the most important changes.

A plugin must be able to expose actual native objects with independent instances.

Example:

```text
native class VideoPlayer {
    init(options: PlayerOptions = {})

    async fn prepare(url: String) throws PlayerError

    fn play()
    fn pause()
    fn seek(position: Float64)
    fn dispose()
}
```

Nexa code must be able to do:

```text
let player1 = VideoPlayer()
let player2 = VideoPlayer()

await player1.prepare(url: "video1.mp4")
await player2.prepare(url: "video2.mp4")

player1.play()
player2.play()
```

These MUST represent two real independent native instances.

Do not implement this using a global singleton plus integer IDs like:

```text
createPlayer() -> Int
play(playerId)
pause(playerId)
```

unless internally required for a very specific backend.

From the Nexa language and plugin author point of view, these are actual object instances.

Conceptually:

```text
Nexa player1
    ↓
Native VideoPlayer instance #1

Nexa player2
    ↓
Native VideoPlayer instance #2
```

This is required for:

* video players;
* audio players;
* WebViews;
* camera sessions;
* databases;
* sockets;
* image processors;
* document instances;
* native SDK objects;
* other stateful APIs.

---

# 9. Object identity and lifetime

Design a proper lifetime model for `native class`.

Nexa must know when an object:

* is created;
* is referenced;
* is passed to another native API;
* is used by a component;
* is destroyed.

For resource-heavy objects support explicit:

```text
dispose()
```

or an equivalent language feature.

Examples include:

* AVPlayer;
* ExoPlayer;
* cameras;
* WebViews;
* sockets;
* native C++ objects;
* large native buffers.

Do not rely exclusively on Kotlin garbage collection for resources requiring deterministic cleanup.

Generated code must protect against:

* use-after-dispose;
* double disposal;
* invalid object references.

Prefer compile-time safety where possible and minimal runtime checks where necessary.

---

# 10. Generate native contracts for plugin authors

One of the best ideas from Nitro Modules is generating native specs that the platform compiler validates.

Use this idea, but adapt it to Nexa's architecture.

For example:

```text
native class VideoPlayer {
    readonly property isPlaying: Bool

    fn play()
    fn pause()
}
```

should generate an iOS contract conceptually similar to:

```swift
protocol NexaVideoPlayerSpec: AnyObject {
    var isPlaying: Bool { get }

    func play()
    func pause()
}
```

The plugin author then implements:

```swift
final class VideoPlayerImpl: NexaVideoPlayerSpec {
    ...
}
```

If the implementation is missing a member or uses the wrong type, the Swift compiler must reject it.

Generate an Android interface such as:

```kotlin
interface NexaVideoPlayerSpec {
    val isPlaying: Boolean

    fun play()
    fun pause()
}
```

and the implementation:

```kotlin
class VideoPlayerImpl : NexaVideoPlayerSpec {
    ...
}
```

The Kotlin compiler should perform the implementation validation.

Prefer Kotlin `interface` instead of forcing plugin authors to extend a Nexa abstract base class.

A plugin implementation may already need to inherit from another SDK type.

Do not consume its inheritance slot unnecessarily.

---

# 11. Constructors

Constructors must be represented in the IDL.

Example:

```text
native class VideoPlayer {
    init(options: PlayerOptions)
}
```

Generate appropriate factories or compile-time glue.

Do not make the architecture dependent on Kotlin interfaces supporting constructors.

For Kotlin, generate factory validation if necessary.

Conceptually:

```kotlin
internal fun createVideoPlayer(
    options: PlayerOptions
): NexaVideoPlayerSpec {
    return VideoPlayerImpl(options)
}
```

If the expected implementation cannot be constructed, compilation should fail with a clear error.

---

# 12. Add native properties

Support native properties explicitly.

Example:

```text
native class VideoPlayer {
    readonly property state: PlayerState
    readonly property duration: Float64

    property volume: Float64
}
```

Generate natural Swift and Kotlin property APIs where possible.

Avoid transforming every property into verbose getter/setter functions unless necessary internally.

---

# 13. Add typed errors

The current handling of native `Result` must be reviewed.

Never silently transform a native failure into:

```text
false
0
""
[]
null
```

unless that behavior was explicitly requested.

Introduce typed errors.

Example:

```text
error PlayerError {
    invalidUrl
    unsupportedFormat
    network
    decodingFailed(message: String)
}
```

Then:

```text
native class VideoPlayer {
    async fn prepare(url: String) throws PlayerError
}
```

Nexa must require explicit error handling according to Nexa language semantics.

On Swift, generated code should map naturally to:

```swift
try await ...
```

On Kotlin, choose a predictable generated representation.

The same Nexa-level error semantics must behave consistently across platforms.

---

# 14. Add events

Events must be first-class in `native.nxid`.

Example:

```text
event stateChanged(state: PlayerState)
event ended()
event progressChanged(position: Float64, duration: Float64)
```

Events from `native class` must belong to the specific object instance.

Therefore:

```text
player1.stateChanged
```

must not interfere with:

```text
player2.stateChanged
```

Avoid global event buses.

Generate efficient native callbacks/observation appropriate for each platform.

---

# 15. Add native visual components

Community plugin authors must be able to expose their own native UI components.

Introduce:

```text
native component
```

Example:

```text
native component VideoView {
    prop player: VideoPlayer

    prop controls: Bool = true
    prop resizeMode: ResizeMode = fit

    event tapped()
}
```

Nexa code should then be able to use:

```text
VideoView(
    player: player1,
    controls: true
)
```

The generated iOS output should integrate directly with SwiftUI.

The generated Android output should integrate directly with Jetpack Compose.

Conceptually:

```text
Nexa component
    ├── SwiftUI View
    └── Compose @Composable
```

Do NOT introduce an unnecessary runtime component registry.

Resolve components statically during compilation.

---

# 16. Separate stateful objects from their visual components

For APIs such as video, camera, maps, WebView, etc., prefer:

```text
state/controller object
+
visual component
```

Example:

```text
VideoPlayer
+
VideoView
```

instead of putting all playback methods on the visual component.

This enables:

```text
let player1 = VideoPlayer()
let player2 = VideoPlayer()

VStack {
    VideoView(player: player1)
    VideoView(player: player2)
}
```

The same model should work for future APIs such as:

```text
CameraSession + CameraPreview
MapController + MapView
WebViewController + WebView
PdfDocument + PdfView
```

---

# 17. Native UI interoperability

Plugin authors must not be limited to purely SwiftUI or purely Compose SDKs.

On iOS allow plugin authors to wrap:

* `UIView`;
* `UIViewController`;

using normal SwiftUI interoperability mechanisms.

On Android allow plugin authors to wrap traditional Android Views through Compose interoperability.

Nexa does not need to reinvent UIKit/View interoperability.

The generated plugin layer should allow authors to use native platform mechanisms naturally.

---

# 18. Remove `abi.nxabi` from normal plugins

Remove the requirement for:

```text
abi.nxabi
```

for normal Swift/Kotlin plugins.

It should not be part of the normal Nexa ↔ native plugin architecture.

Delete or deprecate the current manually authored C ABI mechanism if nothing externally depends on it.

Also remove normal generation commands such as:

```text
nexa plugin generate --target c
```

if the C target is no longer part of the desired architecture.

The Nexa plugin author should not manually describe:

```text
borrowed_readonly
caller_owned
lifetime call
calling_convention c
```

for a standard Swift/Kotlin plugin.

The generator should infer native representation from the IDL.

---

# 19. Remove Rust as a first-class plugin target

For Plugins v2, simplify the supported implementation model.

Primary implementations:

```text
Swift
Kotlin
```

Optional shared native implementation:

```text
C++
```

Remove Rust/C from the public plugin architecture unless the existing repository has a strong concrete reason to preserve them.

Do not maintain complexity purely for hypothetical future use.

If needed later, Rust can return through a dedicated extension architecture.

---

# 20. C++ must be optional, not mandatory

Do NOT make every native plugin go through C++.

Most community SDK plugins will naturally look like:

```text
iOS SDK → Swift
Android SDK → Kotlin
```

Forcing C++ into that path would make Nexa harder to use.

Normal path:

```text
iOS:
Nexa → Swift → plugin implementation

Android:
Nexa → Kotlin → plugin implementation
```

C++ should only be used when the plugin author wants:

* shared cross-platform native logic;
* high-performance algorithms;
* existing C++ libraries;
* video decoding;
* audio DSP;
* image processing;
* ML;
* cryptography;
* compression;
* game engines;
* database engines.

---

# 21. Optional C++ architecture

When a plugin chooses a C++ implementation, the plugin author should write C++, not bridge boilerplate.

Conceptually:

```text
native.nxid
      ↓
generated C++ contract
      ↓
C++ implementation
      │
 ┌────┴─────────────┐
 ↓                  ↓
iOS                Android
 ↓                  ↓
Swift/C++      generated JNI
interop             ↓
 ↓                Kotlin
Swift
```

On iOS:

```text
Nexa
  ↓
Swift
  ↓
Swift/C++ interoperability
  ↓
C++
```

Do not add a C wrapper if modern Swift/C++ interoperability can support the required API.

On Android:

```text
Nexa
  ↓
Kotlin
  ↓
generated JNI layer
  ↓
C++
```

JNI is acceptable internally because Android requires a native boundary.

However:

**the plugin author must not manually write JNI for standard Nexa plugins.**

Generate:

* native declarations;
* JNI entry points;
* object handles if needed;
* type conversion;
* exception conversion;
* lifecycle glue.

The C++ plugin author should ideally only implement generated C++ interfaces.

---

# 22. C++ error safety

Generated C++ boundaries must catch native exceptions before they escape into Swift or JNI.

Conceptually:

```text
C++ exception
      ↓
generated Nexa adapter
      ↓
typed Nexa/native error
      ↓
Swift/Kotlin
```

Do not allow uncaught C++ exceptions to cross a language boundary.

---

# 23. Memory handling should be generated

Do not expose low-level ABI concepts to normal plugin authors.

The author should write:

```text
fn decode(data: Bytes) -> Frame
```

not:

```text
bytes borrowed_readonly
returned_buffers caller_owned
lifetime call
```

Nexa should derive the best safe representation.

The compiler/code generator should determine when it can:

* borrow memory;
* avoid copies;
* move ownership;
* retain data;
* copy data.

Optimize common data paths automatically.

Add internal ABI metadata if required, but it should normally be generated rather than authored manually.

---

# 24. Threading model

Design a threading model before community plugins depend on unspecified behavior.

Support concepts such as:

```text
@main
@background
@any
```

or another clear equivalent.

For example:

```text
@main
fn present()

@background
async fn decode(data: Bytes) -> Frame

@any
fn hash(data: Bytes) -> Bytes
```

Native visual components should default to the UI/main thread.

Map this naturally to native platform concepts such as:

```text
Swift → MainActor
Kotlin → Main dispatcher / Compose rules
```

Avoid hidden thread hopping when possible.

Document the threading guarantee in the generated contract.

---

# 25. Native dependencies

Community plugins need native dependency management.

Add native dependency declarations to `plugin.config.nx`.

Support iOS dependencies through Swift Package Manager.

Conceptually:

```text
ios {
    dependencies {
        swiftPackage {
            url: "https://example.com/vendor/sdk"
            from: "2.0.0"
            products: ["VendorSDK"]
        }
    }
}
```

Support Android Maven/Gradle dependencies.

Conceptually:

```text
android {
    dependencies {
        maven: [
            "com.vendor:sdk:2.0.0"
        ]
    }
}
```

Integrate these dependencies automatically into generated projects.

Detect dependency conflicts early and report useful errors.

---

# 26. Native platform configuration

`plugin.config.nx` should eventually be capable of declaratively describing platform requirements.

Examples:

```text
ios {
    minVersion: "17.0"

    permissions {
        camera
        microphone
    }

    frameworks {
        AVFoundation
    }
}
```

and:

```text
android {
    minSdk: 26

    permissions {
        camera
        recordAudio
    }
}
```

Design the architecture so it can later support:

* Info.plist entries;
* AndroidManifest entries;
* Apple entitlements;
* privacy manifests;
* system frameworks;
* linker flags;
* ProGuard/R8 rules;
* XCFramework dependencies;
* AAR dependencies;
* native resources;
* C++ compiler settings.

Prefer declarative configuration over arbitrary project mutation scripts.

An advanced escape hatch may exist later, but it must not be the default path.

---

# 27. Plugin assets

Assets must work for all plugins, not only pure Nexa plugins.

Support:

```text
assets/
```

for mixed/native plugins.

Make asset resolution deterministic.

Include assets in:

* build planning;
* fingerprints;
* incremental rebuild checks;
* packaging;
* generated iOS resource configuration;
* generated Android resource configuration.

---

# 28. Plugin namespaces and exports

Review the current implicit namespace/interface matching logic.

Avoid rules such as:

```text
if interface name == plugin alias
```

or:

```text
if there is exactly one interface, use it
```

where possible.

The plugin alias should represent the package namespace.

Example:

```text
plugin "../VideoPlugin" as Video
```

could expose:

```text
Video.VideoPlayer
Video.VideoView
Video.PlayerState
```

or a cleaner naming model based on Nexa language conventions.

Exports should be explicit and deterministic.

Do not depend on filename/interface-name guessing.

---

# 29. Static resolution only

Preserve one of Nexa's strongest properties:

**plugins should be statically resolved at compile time.**

Avoid systems like:

```text
getPlugin("VideoPlayer")
```

or:

```text
registry["VideoPlayer"]
```

unless an explicitly dynamic plugin feature is introduced someday.

Prefer generated code such as:

```swift
let player = VideoPlayerImpl(...)
```

and:

```kotlin
val player = VideoPlayerImpl(...)
```

The generated Swift protocol/Kotlin interface should mainly be used to validate the plugin implementation.

Do not introduce dynamic dispatch unnecessarily when the compiler knows the concrete implementation.

---

# 30. Improve dead-code elimination and pruning

The current compiler apparently already removes unused plugins.

Preserve this and make it more granular.

Aim for:

```text
plugin
  ↓
export
  ↓
native class/component
  ↓
member
  ↓
generated adapter
```

If possible, do not generate wrappers for completely unused native APIs.

Example:

A plugin exposes:

```text
Camera
QRScanner
DocumentScanner
FaceDetector
```

but the application only uses:

```text
QRScanner
```

Nexa should avoid generating unused glue where possible.

Do not sacrifice correctness or native dependency rules merely to achieve theoretical pruning, but perform all safe compile-time elimination.

---

# 31. Plugin configuration from `nexa.config.nx`

Preserve the current idea where application-specific plugin settings are provided from:

```text
nexa.config.nx
```

For example:

```text
config {
    plugins {
        Video {
            cacheSize: 128
        }
    }
}
```

Allow the plugin to declare the expected config schema.

Conceptually:

```text
settings {
    cacheSize: Int32 = 64
    autoplay: Bool = false
}
```

Then validate configuration at compile time.

Generate native constants whenever possible.

Avoid runtime dictionaries and JSON config parsing.

---

# 32. Complete `native.nxid` example

The new system should eventually support something around this level of expressiveness:

```text
schema 2

enum PlayerState {
    idle
    preparing
    ready
    playing
    paused
    ended
}

enum ResizeMode {
    fit
    fill
    stretch
}

struct PlayerOptions {
    autoplay: Bool = false
    muted: Bool = false
    volume: Float64 = 1.0
}

error PlayerError {
    invalidUrl
    unsupportedFormat
    network
    playback(message: String)
}

interface Playable {
    fn play()
    fn pause()
}

native class VideoPlayer implements Playable {
    init(options: PlayerOptions = {})

    readonly property state: PlayerState
    readonly property duration: Float64

    property volume: Float64

    async fn prepare(url: String) throws PlayerError

    fn play()
    fn pause()
    fn seek(position: Float64)

    event stateChanged(state: PlayerState)
    event ended()

    fn dispose()
}

native component VideoView {
    prop player: VideoPlayer

    prop controls: Bool = true
    prop resizeMode: ResizeMode = fit

    event tapped()
}
```

Do not blindly implement this exact grammar if it conflicts with good Nexa language design.

Treat it as the semantic target.

---

# 33. VideoPlayer must be the reference integration test

Use VideoPlayer as the main end-to-end test plugin.

Implement it using:

```text
iOS → AVPlayer
Android → Media3 / ExoPlayer
```

The plugin must support at minimum:

```text
let player1 = VideoPlayer()
let player2 = VideoPlayer()
```

with both players existing at the same time.

Test:

```text
player1.prepare(video1)
player2.prepare(video2)

player1.play()
player2.play()
```

and render:

```text
VideoView(player: player1)
VideoView(player: player2)
```

Both players and views must remain independent.

Test destroying only one player while the second continues functioning.

This scenario is a hard requirement, not an optional demo.

---

# 34. Generated project integration

Review and update both generators.

For iOS ensure Nexa correctly handles:

* generated Swift contracts;
* plugin Swift sources;
* SwiftUI components;
* SPM dependencies;
* frameworks;
* resources;
* optional C++;
* Swift/C++ interoperability settings;
* minimum platform requirements.

For Android handle:

* generated Kotlin contracts;
* plugin Kotlin sources;
* Compose components;
* Maven dependencies;
* resources;
* manifests;
* optional C++;
* NDK/JNI generation;
* native libraries;
* minimum SDK requirements.

Generated projects must remain deterministic.

---

# 35. Cache correctness

Review the current incremental compilation fingerprints.

All inputs capable of changing generated output must participate in cache invalidation.

At minimum:

```text
plugin.config.nx
plugin.nx
native.nxid
Swift plugin sources
Kotlin plugin sources
C++ sources
assets
dependency definitions
platform configuration
compiler/codegen version
```

The current situation where some plugin files are validated but not correctly represented in cache fingerprints must not remain.

Add regression tests for this.

---

# 36. Diagnostics

Developer experience is critical.

Plugin errors should be precise.

Bad:

```text
Plugin generation failed.
```

Good:

```text
VideoPlayer/native.nxid:24:5

VideoPlayer.prepare declares:
    async fn prepare(url: String) throws PlayerError

but VideoPlayerImpl on Android does not implement a compatible method.

Expected:
    suspend fun prepare(url: String)

Found:
    fun prepare(url: Int)
```

Where possible connect errors back to:

* `.nxid` declaration;
* Swift implementation;
* Kotlin implementation;
* plugin configuration.

Community authors should not need to understand Nexa compiler internals to fix a plugin.

---

# 37. CLI redesign

Review plugin CLI commands.

A good workflow should eventually resemble:

```text
nexa plugin init MyPlugin
```

which creates the project structure.

Then:

```text
nexa plugin generate MyPlugin
```

generates platform contracts/glue.

Then:

```text
nexa plugin check MyPlugin
```

performs:

```text
plugin.config.nx validation
native.nxid validation
Nexa source validation
Swift contract validation
Kotlin contract validation
native dependency validation
```

Consider whether explicit `--target swift/kotlin` is still needed.

Prefer generating all required platform contracts by default when practical.

Remove obsolete C/Rust generator commands.

---

# 38. Test strategy

The current plugin architecture needs much stronger automated testing before becoming a community API.

Add dedicated tests for at least:

## IDL parser

Test:

* structs;
* enums;
* interfaces;
* services;
* native classes;
* native components;
* constructors;
* properties;
* async;
* errors;
* events;
* optional values;
* collections;
* invalid syntax;
* invalid recursive types;
* invalid references.

## Swift code generation

Use golden/snapshot tests.

Test generated:

* protocols;
* structs;
* enums;
* factories;
* components;
* async APIs;
* throwing APIs;
* events.

## Kotlin code generation

Equivalent test coverage.

## Native conformance

Create intentionally invalid plugins.

Examples:

* missing method;
* wrong argument type;
* wrong return type;
* missing property;
* wrong mutability;
* incorrect constructor/factory.

Confirm native compilation fails clearly.

## Object instances

Test:

```text
object1 != object2
```

and independent state.

## Lifecycle

Test:

```text
create
use
dispose
create again
```

## Native components

Render at least two independent native components simultaneously.

## Errors

Ensure the same Nexa-level semantics exist on both platforms.

## Events

Ensure events are scoped to their instance.

## Plugin configuration

Test:

* defaults;
* overrides;
* invalid types;
* missing required settings.

## Dependencies

Test:

* valid dependencies;
* conflicts;
* platform-specific dependencies.

## Cache

Changing any relevant plugin file must invalidate the correct stage.

## Pruning

Unused plugins/exports/glue should not be unnecessarily emitted.

## End-to-end

Compile actual small iOS and Android applications containing community-style plugins.

---

# 39. Migration strategy

Do not perform the redesign as one uncontrolled rewrite.

Implement it incrementally.

Suggested phases:

## Phase 1 — Audit and architecture

* map the current implementation;
* identify affected compiler modules;
* identify parser/codegen/build changes;
* write the Plugins v2 internal architecture document;
* define migration risks.

Do not make major code changes before this is complete.

## Phase 2 — `plugin.config.nx`

* introduce the new manifest;
* make it the actual source of truth;
* update plugin discovery;
* update fingerprints;
* remove reliance on JSON;
* keep temporary legacy parsing only if required for repository migration.

## Phase 3 — `native.nxid` parser/model

Add:

```text
struct
enum
interface
service
native class
error
```

and proper resolved type checking.

## Phase 4 — generated native contracts

Generate:

```text
Swift protocols/types
Kotlin interfaces/types
```

and validate implementations through native compilers.

## Phase 5 — object instances

Implement:

```text
native class
constructors
properties
lifetime
dispose
```

Replace the assumption that every native plugin is a singleton.

VideoPlayer multi-instance must pass here.

## Phase 6 — async/errors/events

Implement consistent:

```text
async
throws
event
```

semantics.

## Phase 7 — native components

Implement:

```text
native component
```

for:

```text
SwiftUI
Jetpack Compose
```

Use VideoView as the first reference component.

## Phase 8 — dependency/config integration

Add:

```text
SPM
Maven/Gradle
permissions
resources
platform requirements
```

## Phase 9 — optional C++

Add only after the Swift/Kotlin architecture is stable.

Implement:

```text
Swift/C++ on iOS
generated JNI on Android
```

Do not expose JNI to normal plugin authors.

## Phase 10 — cleanup

Remove obsolete:

```text
nexa.plugin.json
interfaces.nxid
abi.nxabi
C generator
Rust generator
legacy singleton assumptions
obsolete code paths
```

Update documentation and examples.

---

# 40. Backward compatibility

Do not keep bad architectural decisions indefinitely solely for backward compatibility.

However, do not silently break existing projects either.

During development determine whether we should:

1. provide a temporary migration layer;
2. provide an automatic migration command;
3. intentionally make Plugins v2 a breaking schema/version.

Prefer clear versioning over permanent compatibility complexity.

For example:

```text
schema 1 → legacy plugin system
schema 2 → Plugins v2
```

If a legacy plugin is encountered, provide a precise migration diagnostic.

---

# 41. Architecture principles

Throughout the implementation follow these principles.

### Compile-time over runtime

If Nexa can know something during compilation, do not rediscover it at runtime.

### Native language first

Swift developers should write natural Swift.

Kotlin developers should write natural Kotlin.

C++ developers should write natural C++.

Do not force them to understand unrelated bridge internals.

### Generated boilerplate

If glue code is deterministic, generate it.

Plugin authors should not manually write:

```text
JNI
C wrappers
registries
serialization glue
native handles
binding boilerplate
```

unless they intentionally opt into low-level control.

### Zero-cost when possible

Do not introduce abstraction layers that remain at runtime when they can disappear during generation.

### Explicit contracts

Avoid ambiguous magic.

Prefer explicit IDL and clear diagnostics.

### Multiple instances by design

Any stateful object must naturally support multiple simultaneous instances.

### UI is first-class

Native visual components are part of the plugin model, not an afterthought.

### Platform escape hatches

Do not make the abstraction so strict that real-world SDKs become impossible to integrate.

Allow platform-specific implementation where appropriate.

---

# 42. Things NOT to do

Do not:

* copy Nitro Modules architecture blindly;
* add C++ simply because Nitro uses C++;
* force every plugin through a C ABI;
* create a runtime plugin registry;
* use JSON serialization for normal native calls;
* use reflection for ordinary plugin resolution;
* represent stateful objects as global singleton plugins;
* use global integer object IDs in the public Nexa API;
* swallow native errors and return default values;
* expose JNI complexity to plugin authors;
* expose manual memory ownership rules unless absolutely necessary;
* implement native visual components as WebViews;
* create separate disconnected systems for every platform if code generation can provide one semantic model;
* over-engineer hypothetical platforms before iOS/Android are correct.

---

# 43. Things to preserve from the current system

If they already work correctly, preserve or improve:

* static plugin resolution;
* AOT generation;
* unused-plugin pruning;
* direct Swift/Kotlin calls;
* compile-time validation;
* `nexa.config.nx` application plugin settings;
* deterministic builds;
* native performance.

The redesign should improve these properties, not replace them with a runtime abstraction.

---

# 44. Expected architecture

The final general architecture should approximately be:

```text
                     plugin.config.nx
                            │
          ┌─────────────────┼──────────────────┐
          │                 │                  │
      plugin.nx         native.nxid         assets/deps
      optional           optional
          │                 │
          │           Native Codegen
          │                 │
          │       ┌─────────┴─────────┐
          │       │                   │
          │       ▼                   ▼
          │   Swift specs        Kotlin specs
          │       │                   │
          │       ▼                   ▼
          │  Swift plugin        Kotlin plugin
          │       │                   │
          └───────┼───────────────────┘
                  │
             Nexa compiler
                  │
           reachability/AOT
                  │
        ┌─────────┴─────────┐
        ▼                   ▼
   SwiftUI app         Compose app
```

Optional C++:

```text
                    native.nxid
                         │
                 generated C++ spec
                         │
                  C++ implementation
                         │
               ┌─────────┴─────────┐
               │                   │
          Swift/C++           generated JNI
               │                   │
             Swift               Kotlin
               │                   │
               └────── Nexa ───────┘
```

Native UI:

```text
native component
       │
       ├── SwiftUI View
       │
       └── Compose @Composable
```

Stateful object:

```text
Nexa object
     │
     └── exact native instance
```

not:

```text
Nexa object
     ↓
global plugin singleton
     ↓
manual integer ID map
```

---

# 45. Required final deliverables

Work through the repository and produce the following.

## A. Current architecture report

Explain:

* how plugins currently work;
* important source files/modules;
* limitations;
* technical debt;
* assumptions that must change.

## B. Plugins v2 technical design

Document:

* package model;
* `plugin.config.nx`;
* `native.nxid`;
* IDL AST;
* type system;
* services;
* native classes;
* components;
* errors;
* events;
* async model;
* threading;
* lifetime;
* dependencies;
* C++ option;
* code generation.

## C. Migration map

For every major current subsystem state:

```text
current implementation
→ replacement
→ files/modules affected
→ migration risk
```

## D. Implementation

Implement the architecture progressively following the phases above.

Do not stop at documentation if the repository can be safely modified.

## E. Automated tests

Add strong tests for every new compiler feature.

## F. VideoPlayer reference plugin

Create or migrate a real VideoPlayer example using:

```text
iOS → AVPlayer
Android → Media3 / ExoPlayer
```

demonstrating:

```text
2 simultaneous VideoPlayer objects
2 simultaneous VideoView components
independent state
independent events
correct disposal
```

## G. Documentation

Create clear community plugin-author documentation.

A plugin author should be able to understand how to create:

1. a pure Nexa plugin;
2. a native service;
3. a stateful native class;
4. a native component;
5. a plugin with native dependencies;
6. an optional C++ plugin.

---

# 46. How to work

Do not make speculative edits without understanding affected compiler paths.

For each implementation phase:

1. inspect the current code;
2. identify the smallest coherent architectural change;
3. implement it;
4. add/update tests;
5. run relevant test suites;
6. fix regressions;
7. remove obsolete code only when the replacement works;
8. summarize what changed before proceeding to the next major phase.

Prefer modifying foundational compiler representations first rather than patching generated output in many separate places.

If the current architecture makes one of these goals difficult, refactor the underlying architecture instead of layering hacks on top.

Do not ask me to choose between minor implementation details when one option is clearly more consistent with the architecture described here.

Use your judgment and document important decisions.

---

# 47. Final objective

The end result should make creating a Nexa native plugin feel approximately like this:

```text
1. Declare native API in native.nxid.

2. Run Nexa code generation.

3. Implement generated Swift protocol.

4. Implement generated Kotlin interface.

5. Optionally implement a SwiftUI/Compose native component.

6. Use the plugin directly from Nexa.
```

For example:

```text
let firstPlayer = VideoPlayer()
let secondPlayer = VideoPlayer()

await firstPlayer.prepare(url: firstUrl)
await secondPlayer.prepare(url: secondUrl)

VStack {
    VideoView(player: firstPlayer)
    VideoView(player: secondPlayer)
}
```

which should compile approximately into:

```text
iOS:
Nexa
→ generated Swift
→ VideoPlayerImpl #1 / #2
→ AVPlayer
→ SwiftUI VideoViewImpl
```

and:

```text
Android:
Nexa
→ generated Kotlin
→ VideoPlayerImpl #1 / #2
→ ExoPlayer
→ Compose VideoViewImpl
```

No C ABI.

No plugin registry.

No JSON bridge.

No singleton pretending to represent multiple stateful objects.

No unnecessary runtime layer.

The plugin system should feel like a **native extension system for an AOT compiler**, not like a JavaScript/native bridge framework.

That is the architectural target.

