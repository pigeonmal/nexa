# Nexa plugin architecture

Audit date: 2026-09-23

This document describes the Plugins v2 implementation in the repository and
the remaining boundaries. It is an implementation report, not a claim that the
full roadmap is complete.

## Current architecture

An app declares a local package with `plugin "path" as Namespace`. The compiler
resolves `plugin.config.nx` relative to the app, loads optional Nexa source
through the normal source graph, parses optional `native.nxid`, and makes its
signatures available to ordinary semantic checking. The optimized typed IR
retains only native packages referenced by surviving calls or native component
nodes. Project generation then copies the reachable platform sources, declared
assets, generated contracts, compile-time option constants, native dependencies,
and host metadata.

The native path is direct AOT output. Swift calls the concrete
`{Name}Impl`; Kotlin calls its concrete implementation type. Native classes are
ordinary native objects created at each constructor expression, so two Nexa
bindings can refer to two independent instances. Services remain stateless
contracts. Qualified `Namespace.Component(...)` calls lower to direct SwiftUI
and Compose wrappers. No plugin registry, reflection lookup, JSON/RPC bridge,
or mandatory C ABI is present.

| Responsibility | Current source |
| --- | --- |
| Package manifest parser and validation | `crates/nexa-plugin-idl/src/manifest.rs` |
| Native IDL AST, parser, type validation | `crates/nexa-plugin-idl/src/lib.rs` |
| Plugin discovery and `.nx` call lowering | `crates/nexa-compiler/src/project.rs`, `semantic/expressions.rs` |
| Typed plugin declarations in shared IR | `crates/nexa-ir/src/lib.rs` |
| Plugin reachability pruning | `crates/nexa-compiler/src/optimize.rs` |
| CLI init/check/binding generation | `crates/nexa-cli/src/plugin.rs`, `plugin/bindings.rs` |
| Native source, asset, and option integration | `crates/nexa-cli/src/project/plugins.rs` |
| Xcode and Gradle package/host templates | `crates/nexa-cli/src/project/templates.rs` |
| Local reference package | `examples/plugins/video-player/` |

The previous JSON manifest, `interfaces.nxid`, hand-authored `abi.nxabi`, and
C header generator are not current inputs. Schema 2 is the alpha package
format; this repository does not promise an automatic migration for the
superseded formats.

## Plugins v2 contract

`plugin.config.nx` is the package metadata source of truth. It declares package
identity, optional source files, platform source globs, assets, minimum platform
versions, native dependencies, linker arguments, and compile-time host
requirements. iOS entitlements support string, boolean, and string-array
values; conflicting values fail project generation. Linker arguments are
validated as one non-empty argument per entry and escaped in Xcode settings.
Paths are relative to the package and validated before use. App-specific
scalar options remain in the generated project's `nexa.config.nx` and are
emitted as direct native constants.

`native.nxid` is independent of the app grammar. It currently describes
complete value structs, closed enums, typed error variants with payloads,
interfaces, services, native classes, native components, constructors,
properties, events, methods, and scalar compile-time options. Field defaults,
all type references, generic arity, and recursive inline value layouts are
checked before binding generation. Bare opaque `type Name` declarations are
rejected; boundary values must have a described layout or be a declared native
class. Recursive values through `Array`, `Set`, or `Map` remain possible
because those containers break the inline layout cycle.

Native classes map to concrete `{Name}Impl` classes and native properties or
methods. Generated protocol/interface contracts are checked by both native
compilers; the binding source now includes a private unused conformance
function for each class so a missing implementation conformance fails host
compilation without adding a call to the app's runtime path. Swift native-class
contracts are `@MainActor` so stateful UI-bound SDK objects can be used safely
from generated SwiftUI actions. Kotlin calls stay on the caller's coroutine
dispatcher; Nexa does not inject a background dispatcher. Generated Kotlin
factory probes type-check each declared constructor through its `{Name}Impl`
class; the private unused probe adds no app runtime call. Native class
constructor bindings at app, screen, and custom component scopes use SwiftUI
`@StateObject` factory storage or Compose `remember`, so recomposition preserves
object identity without repeatedly constructing native resources. Mutable
Swift bindings write through the same identity storage. Readonly
properties are accessed directly, and mutable native properties support typed
assignment in event actions; readonly assignments are rejected during semantic
analysis.

`nexa plugin check` parses the manifest and IDL, syntax-checks the declared
Nexa source import graph, and verifies that declared platform source patterns
and asset roots resolve within the package. When Xcode and the iOS simulator
SDK are available, it invokes Swift 6 type-checking over the declared Swift
sources and generated contract. Kotlin conformance remains a host Gradle build
check when the implementation uses Maven dependencies, external imports, or
native Compose components. For dependency-free service and class plugins,
`nexa plugin check` invokes `kotlinc` against the generated contract when the
compiler is installed; the full host Gradle build remains authoritative for
Android classpaths.

Native components are statically qualified by their plugin namespace. Required
properties are forwarded to a SwiftUI `View` wrapper or Compose function;
literal defaults declared on native component properties are inserted during
semantic lowering when a call omits the property. IDL validation rejects
mismatched defaults and defaults on non-component properties. Generated Swift
and Kotlin wrappers also expose platform-safe literal defaults. A component may
declare one `content` slot; Nexa then requires a child block and forwards the
lowered views through the native builder closure. Components without that
declaration reject child blocks.
Component events attach with `.onEvent { ... }` modifiers and become direct
callback arguments to that call's wrapper. Native-class events are subscribed to on their concrete instance with
`player.ended { ... }`; payload events use
`player.progressChanged { position, duration -> ... }`. Semantic checking
validates event identity, payload count and types, and generated code assigns
the handler to that instance's callback property. Callback properties hold one
handler, so a later assignment replaces an earlier handler. IDL errors preserve
named variants and payload types in generated contracts: Swift uses
associated-value `Error` enums and Kotlin uses sealed `Exception` subclasses.
`throws` and `Result<Success, Failure>` must use declared error types. Async
throwing calls in action blocks require `try { ... } catch { ... }`; Swift
preserves them with typed `throws(ErrorType)` and `do`/`catch`, and Kotlin uses
`@Throws`, sealed error classes, and `try`/`catch`. Catch arms bind declared
variants and payloads. Cases must cover all variants for a single typed error;
an `else` arm is required for untyped errors or multiple error types.

The regular Swift/Kotlin implementation path has no hidden background
dispatch. iOS native class methods are main-actor isolated; Kotlin methods stay
on the caller's dispatcher, so plugin authors must switch dispatchers when an
SDK requires it. Generated C++ async wrappers are the exception: iOS waits on
futures off the main queue, and Android runs blocking JNI/future calls on
`Dispatchers.IO`.
Native class `dispose()` methods must be synchronous, non-throwing,
parameterless `Void` methods. Within each action sequence, semantic lowering
tracks explicit disposals through sequential statements and conservatively
joins conditional and loop paths; it rejects duplicate disposal and later
uses. Immutable direct aliases of native-class bindings share the same
disposal identity within an action sequence. A mutable source binding cannot
be replaced while such an alias may still point to the disposed instance.
Callback analysis also rejects disposal in one UI/lifecycle callback when a
different callback can use or dispose the same declared instance. It connects
app and screen state bindings across those scopes, and component-local states
and parameters within each component. `OnDisappear` disposal is treated as
terminal cleanup for that view lifetime. Native-class values passed to
rendered component calls count as uses for the containing view, so a separate
UI event cannot dispose an object still mounted in the view tree. Object
parameters in custom components are borrowed and cannot dispose the passed
object; nested component parameters forward that same native reference, while
the owning app or screen remains responsible for disposal. Screen
`OnDisappear` can dispose only screen-owned objects, so it cannot release an
app-owned instance still used by another route. Each screen destination has its
own state lifetime: Compose scopes state to its back-stack entry, and SwiftUI
uses a unique route identity for each destination. Route parameters stay scalar;
reusing resources across separate route lifetimes remains open.
Plugin implementations should clear callbacks they retain. C++ contracts,
opt-in source compilation, and direct Swift/C++ ownership are implemented.
Both platforms adapt synchronous and non-throwing async scalar, optional scalar,
string, and byte service/class methods; iOS waits on futures off the main queue,
and Android uses `Dispatchers.IO`. Android classes require `dispose()` for
deterministic release. iOS translates declared async typed errors from C++
`NexaResult` values for supported scalar, string, and byte results. Android
translates async `NexaResult` failures into the declared Kotlin sealed
exception cases for supported scalar, string, and byte payloads. Async methods
also accept supported collection parameters and return values, with collection
conversion performed on the existing background execution path. Native-class
C++ events use per-instance callbacks; Swift delivers them on the main actor and
Android posts them to the main looper. Callback replacement and disposal
deactivate pending deliveries and release retained callback state.

iOS bridges optional primitive, string, and byte values; flat primitive,
string, and byte arrays; compatible sets; recursively nested arrays; and maps
with supported keys and nested collection values. Swift `String`/`Data`
conversions are explicit, and vector façades preserve compatible `std::set`
semantics. Android bridges optional scalars, nested arrays, compatible sets,
maps with primitive/string keys and supported primitive, string, byte, array,
set, or nested-map values, and nullable outer maps. JNI uses length-aware
string/byte conversions and preserves unsigned bit patterns. Platform-specific
collection combinations outside those supported shapes remain open.

## Migration map

| Earlier boundary | Plugins v2 boundary | Main files | Risk / status |
| --- | --- | --- | --- |
| JSON package metadata and convention probing | Validated `plugin.config.nx` | `manifest.rs`, compiler project loader, CLI cache/scaffolder | Implemented; schema 2 is a breaking alpha boundary. |
| `interfaces.nxid` and opaque type names | `native.nxid` with structured value types and native declarations | `nexa-plugin-idl`, generated bindings | Implemented; opaque `type Name` syntax is now removed. |
| Singleton assumption for every native API | Stateless `service` and concrete `native class` instances | semantic expression lowering, both backends | Constructors, direct methods, mutable properties, instance-scoped class events, per-entry route state, and multi-instance example implemented; callback disposal checks follow app/screen ownership and borrowed component identities. |
| One generic interface for native UI | Qualified `native component` declarations | semantic lowering, both component generators | Required and defaulted properties, declared child slots, direct wrappers, and per-call event subscriptions implemented. |
| Hand-authored C bridge | Generated Swift/Kotlin contracts and direct calls | `plugin/bindings.rs`, project plugin copying | Normal C ABI path removed; optional C++ adapters cover synchronous scalar, optional, string, and byte values plus flat arrays and compatible sets on iOS, and optional values, flat arrays, compatible sets, and flat scalar/string maps on Android. |
| Implicit native build defaults | Manifest-declared C++ language minimum | plugin manifest, IR, Xcode and CMake generation | `cpp.standard` validates C++17/20/23 and applies the highest reachable requirement to generated iOS and Android targets; default remains C++20. |
| Package sources inferred from fixed folders | Manifest-declared source globs and asset paths | manifest parser, project plugin copier, cache | Implemented; Android currently requires one implementation package per plugin. |
| Host dependencies inferred ad hoc | Typed SwiftPM/Maven declarations, iOS frameworks and XCFrameworks, Android HTTPS repositories and AARs, platform resources, privacy manifests, R8 rules, and platform minima | IR plugin metadata, `project/templates.rs`, plugin artifact copying | Implemented with conflict checks and package-bound path validation; native package managers write lockfiles during dependency resolution. |
| App config as runtime option dictionary | Typed `nexa.config.nx` validation and generated constants | `crates/nexa-cli/src/config.rs`, plugin emitters | Implemented for scalar values. |
| No plugin-owned host permission metadata | iOS usage descriptions, signing entitlements, linker flags and privacy manifests, plus Android permission names and R8 rules in package config | manifest parser, IR, native project templates | Implemented for usage descriptions, string/boolean/string-array entitlements, ordered linker arguments, privacy manifests, and Android permissions/rules. |

## Native conformance validation

`nexa plugin check` type-checks Swift implementations with Xcode when the
package has no SwiftPM dependencies, and
dependency-free Kotlin service/class implementations with `kotlinc` when those
toolchains are available. Android APIs, Maven dependencies, and Compose
components require the generated Gradle host build. SwiftPM dependencies are
resolved by the generated Xcode project.
The compiler-conditional headless tests verify that a `Float64` parameter
is emitted with the canonical Swift/Kotlin `Double` spelling, that matching
implementations type-check, and that an implementation using `Float` is
rejected. Typed Kotlin error payloads also compile when their IDL field is named
`message`, which overrides the inherited `Throwable.message` property. The
generated VideoPlayer Android host is assembled with Gradle when an API 37 SDK
and a cached or installed Gradle distribution are available. Android C++
adapters are exercised by a host-JVM JNI smoke test, including embedded NUL and
supplementary Unicode strings; the generated bridge and implementation are
cross-compiled and linked with the NDK when available. Xcode builds also
type-check generated Swift `Data` to C++ byte-vector adapters and set facades.

## Remaining work

1. Complete the remaining generated C++ host adapter cases. Both platforms
   support non-throwing async scalar, optional scalar, string, and byte methods;
   iOS also translates declared typed errors for supported scalar/string/byte
   results. Android supports async typed errors with non-optional primitive,
   string, and byte payloads. Native-class C++ events are supported on both
   platforms; collection combinations outside the covered shapes remain open. Android map keys
   exclude floating-point values, and nullable maps are
   supported only at the outer map level.
2. Reuse native resources across separate route lifetimes; route parameters
   currently stay scalar.

The VideoPlayer example includes headless Swift and Android/JVM smoke tests for
independent instance behavior and deterministic disposal using fake media
engines. Generated projects also compile the native bridges. Device-level UI,
playback, and event delivery are not yet covered.
