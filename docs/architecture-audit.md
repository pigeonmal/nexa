# Nexa architecture audit

Audit date: 2026-09-23

This audit was performed against the Rust workspace, the CLI project
scaffolder, both native backends, and representative generated projects for
minimal UI, lists, navigation, permissions, images, network calls, and plugins.
The initial pass was read-only. The second section records the priority fixes
implemented after the findings were reviewed.

## Findings before the fixes

### P0: plugin model and backend capability ownership

- Plugins were limited to a local `native.nxid` plus native source trees.
  Pure Nexa packages, package-owned assets, and source-graph loading were not
  represented.
- Swift and Kotlin each independently scanned the IR for Network, Path, File,
  and remote-image usage. A future backend could easily disagree and add an
  optional dependency that the app does not use.
- Native generation returned one large source string per platform. Plugin Swift
  files were concatenated into that file, changing Swift `private` boundaries
  and preventing independent native compilation.

### P1: generated native hot paths

- iOS FastList reconfigured all visible rows on every SwiftUI update and used a
  `map(...).min()` allocation on each table scroll event.
- Android network calls created a temporary header map and a request client for
  each call. Cronet callback completion paths were not unified behind one
  terminal guard.
- Each generated remote-image call site could create its own Coil ImageLoader,
  memory cache, and Cronet adapter.
- iOS remote images decoded the complete response without a byte limit or
  downsampling step.
- Cronet provider installation only rendered app content from the completion
  callback and ignored installation failure, which could leave a blank screen.

### P2: ABI and measurement gaps

- The IDL binding generator emits direct Swift/Kotlin declarations, but no
  versioned C++/Rust ABI, buffer ownership contract, or zero-copy adapter.
- There was no repeatable final native size report for APK/AAB R8 output or
  Swift binary/resource output.

## Priority fixes implemented

- Added `nexa_ir::capabilities::analyze` and made both backends consume the
  same post-optimization capability result for core API pruning.
- Added `nexa plugin init --kind pure`. Pure packages contain `plugin.nx` and
  optional `assets/`, are loaded through the normal source graph, and remain
  subject to component reachability pruning.
- Added deterministic plugin asset copying to Android resources and an iOS
  asset catalog. Reachable native Swift plugin files remain separate Xcode
  source inputs.
- Fixed cache fingerprinting for pure plugin source and asset graphs.
- Removed Android request-header list allocations, shared the generated Coil
  ImageLoader per application context, and added a deterministic Cronet provider
  failure UI.
- Added iOS remote-image response limits and ImageIO downsampling.
- Removed avoidable UITableView visible-row mapping and animation closure work
  in FastList updates; table scroll position now uses UIKit's sorted first row.
- Replaced the JSON plugin manifest with a validated `plugin.config.nx`
  manifest. It owns package identity, source declarations, platform metadata,
  and asset paths; compiler discovery, CLI checks, and cache fingerprints now
  resolve package contents from that manifest.
- Replaced `interfaces.nxid` with `native.nxid` and removed the normal C header
  and hand-authored `abi.nxabi` workflow. The typed parser now accepts structs,
  enums, errors, services, native classes/components, properties, events,
  constructors, and throwing async methods. Swift/Kotlin binding generation
  emits direct native contracts and value models.
- Added `nexa audit`, which emits a machine-readable report of optimized IR
  capabilities, generated source bytes, and target dependencies. Project
  generation now emits `nexa.sources.json`, an explicit list of generated
  source/resource units.
- Added opt-in `nexa audit --release-sizes` measurements. On hosts with the
  native toolchains, it builds an Android Release APK with R8/resource
  shrinking and a device-optimized iOS Release app, then reports APK DEX,
  resource, asset, and native-library payload sizes plus the iOS executable
  and app-bundle sizes. Missing toolchains and failed builds are explicit in
  the JSON report; the default static audit remains fast and performs no
  native builds.
- Split generated app output into independently compiled native units for
  types, app declarations, components, runtime helpers, native libraries,
  permissions, assets, lists, and functions. The Xcode project includes each
  Swift unit as a separate source input; Android keeps each Kotlin unit in the
  same generated package.
- Replaced the process-global Android Activity permission holder with a
  lifecycle-owned `rememberLauncherForActivityResult` bridge.
- Cached Cronet engines by canonical certificate-pin sets, unified terminal
  completion guards, and moved download sink writes off the Cronet callback
  executor.
- Aligned iOS and Android certificate pinning on SHA-256 hashes of DER-encoded
  SubjectPublicKeyInfo values. iOS checks the already-trusted server chain;
  both targets accept case-insensitive 64-character hex pins, and malformed
  certificate DER fails closed.
- Reorganized both native generators into `api/`, `components/`, and `engine/`
  responsibility folders. Feature modules now declare the imports required by
  their own emitted code; the backend import module only collects, deduplicates,
  orders, and renders those contributions. Adding or replacing one native
  component therefore stays local to its feature module.
- Bumped the CLI native-source cache schema after the backend/import refactor.
  The cache fingerprints source and plugin graphs, so generator-only semantic
  changes must advance `CACHE_VERSION` to prevent stale Swift/Kotlin units from
  being restored. This keeps cache hits deterministic while preserving the
  existing source-graph cache behavior.
- Added native class constructor and instance-method lowering. A `native class`
  constructor now creates a typed object expression, and calls such as
  `player.prepare()` lower to direct receiver calls instead of the service
  singleton path. Read-only properties, mutable property assignments, named
  arguments, and `Void`/async action calls lower directly. Native-class event
  subscriptions now lower to direct instance callback properties with typed
  payload bindings. The compiler also rejects duplicate disposal and possible
  post-disposal use within an action sequence and tracks direct immutable
  aliases to the same object. A separate callback analysis now covers app,
  screen, and component callback groups, with `OnDisappear` treated as terminal
  cleanup for that view lifetime.
- Project generation now emits each used plugin's generated Swift/Kotlin
  contract automatically. Swift contracts remain separate Xcode source units;
  Android contracts are placed beside the implementation package and imported
  by the generated app source. A Kotlin native plugin must currently use one
  implementation package across its declared source files so this placement is
  deterministic.
- Added qualified native visual component calls such as
  `Video.VideoView(player: player, controls: true)`. Semantic lowering checks
  the IDL properties and emits a direct `Node::NativeComponentCall`; generated
  SwiftUI/Compose bindings call the plugin's `{Name}Impl` directly. Child
  content and component event subscription syntax remain intentionally outside
  this first slice. The cache schema is now `build-v61` for this semantic and
  generated-source change.
- Added declarative native dependencies and platform minimums to the manifest.
  Reachable Swift packages become Xcode package/product references; Android
  Maven coordinates enter Gradle, and host deployment minimums rise to satisfy
  plugin requirements. Conflicting versions fail project generation. The cache
  schema advances to `build-v63` for the changed project output.
- Finished Kotlin import ownership for Text, status bar, direction, lifecycle,
  and expression-generated size-class access. Their focused emitters now declare
  their own imports; the shared import set only orchestrates, deduplicates,
  sorts, and renders. The generic component module now dispatches Text rendering
  to its feature file. Cache schema `build-v63` also invalidates prior generated
  output after this backend module change.
- Tightened Kotlin import gates found by independent review: scroll observers
  now import `LaunchedEffect` for end-reached, scroll-position, and scroll-event
  handlers, while Compose `Image` is emitted only for local image nodes rather
  than icon-only asset helpers. The cache advances to `build-v64` so these final
  generated imports cannot be restored from the previous schema.
- Added backend import regression tests. They pin the minimal SwiftUI/Compose
  baselines, verify that repeated UIKit use is deduplicated, keep Swift network
  and remote-image imports on their owning API paths, distinguish local from
  remote Coil imports, ensure image-only Android output does not gain upload
  support, and verify that the expression generator owns the size-class import.
  They found and fixed an `AsyncImage` import that was incorrectly gated on all
  images instead of remote images. A generated asset-only Android app now
  compiles without Coil or Cronet dependencies. These tests exercise the
  ownership model without adding empty import hooks to SwiftUI-only components.
  The cache schema advances to `build-v65` for the corrected generated import.
- Added compile-time native-class conformance probes to generated Swift and
  Kotlin plugin bindings. They type-check each concrete `{Name}Impl` against
  its IDL protocol/interface without adding a runtime call or changing the
  direct concrete call path. IDL struct fields now validate every referenced
  type and default, and reject recursive inline value layouts while retaining
  collection-indirected trees. Layout-less opaque `type Name` declarations
  are rejected.
- Extended schema-2 plugin metadata with iOS `usageDescriptions` and Android
  `permissions`. Project generation merges reachable declarations into
  `Info.plist` and `AndroidManifest.xml`, deduplicates matching declarations,
  and rejects conflicting iOS purpose text. Swift native-class contracts now
  use `@MainActor`, and throwing async `Void` calls no longer receive an
  invalid value fallback. Mutable native properties now lower to direct writes,
  with readonly writes rejected by semantic analysis and covered by regression
  tests. Calls to throwing plugin APIs in action blocks now require explicit
  `try`/`catch` recovery, preserving native failures instead of silently
  substituting values. At that stage catch blocks did not yet bind or match error variants.
  The native-source cache schema is `build-v69` so
  earlier generated contracts, validations, and statements are invalidated.
- Expanded headless regression coverage across all nine workspace crates for
  plugin import parsing, IDL type references, shared capability scans, typed
  plugin lowering, native receiver and File API code generation, generated
  value contracts, native signature conformance failures, unused-plugin
  pruning, and cache invalidation. The binding test caught and fixed
  Swift `Float32`/`Float64` emission so those IDL types map to native `Float`/
  `Double`. `nexa plugin check` now syntax-checks the declared Nexa import graph
  and verifies declared platform source globs and asset roots before reporting
  the package as valid. On hosts with Xcode and the iOS simulator SDK, it also
  invokes Swift 6 type-checking against the generated plugin contract; the
  VideoPlayer implementation passes this check. `nexa plugin check` also uses
  `kotlinc` for dependency-free Kotlin implementations when available, and a
  compiler-conditional integration test compiles a generated Kotlin contract
  and implementation.

The plugin binding's floating-point type correction changes generated native
source and therefore advances the cache schema to `build-v70`.

Native-class event lowering and persistent storage for immutable native-class
bindings change generated output, so the current cache schema is `build-v72`.
Native-component event modifiers now lower through typed IR, survive
optimization, and render as direct SwiftUI/Compose callback arguments. The
VideoPlayer integration exercises independent callbacks on two component
instances. This generated-source change advances the cache schema to
`build-v73`.
Generated Kotlin contracts also include an unused compile-time factory probe
for each native class. The host compiler therefore verifies constructor
argument and implementation-constructor types even if the app never creates
that class; the probe has no runtime call site.
Plugin manifests now declare iOS system frameworks and HTTPS Android Maven
repositories. Xcode links the declared frameworks directly; generated Gradle
settings merge custom repositories deterministically and enable dependency
locking. This project-output change advanced the cache schema to `build-v74`.
Typed error variants now retain payloads in Swift/Kotlin contracts, and native
class disposal analysis resolves direct immutable aliases. Those generated
contract and semantic validation changes advance the current cache schema to
`build-v75`.
Plugin-owned iOS entitlements now validate and flow into a merged signing
entitlements plist plus the Xcode target's `CODE_SIGN_ENTITLEMENTS` setting.
The generated-project output change advanced the cache schema to `build-v76`.
Qualified stateless service calls now parse correctly inside action blocks,
matching their documented `Namespace.method()` form. The compiler behavior
change advances the cache schema to `build-v77`.
iOS linker arguments now flow as individually escaped `OTHER_LDFLAGS` values
in declaration order. The project-output change advances the cache schema to
`build-v78`. Recursive source-glob expansion now includes matching files at
every depth; this generated-project change advances the cache schema to
`build-v79`. The generated iOS integration test verifies that the declared
Swift implementation is included and compiles with its generated contract.
Explicit cross-platform `try`/`catch` actions now preserve throwing plugin and
asynchronous File/Network failures, and the VideoPlayer reference contract
exercises a typed invalid-URL failure. This generated-source change advances
the native-source cache schema to `build-v81`. Callback lifetime validation
now treats native-class values passed to rendered components as live for the
containing view; the semantic validation update advances the cache schema to
`build-v82`.

Typed `.nx` error cases now bind declared payloads on both targets. The
compiler requires exhaustive cases for a single declared error, or an `else`
fallback for untyped/multiple failures; Swift contracts use typed throws so
unhandled errors cannot escape SwiftUI's non-throwing task closure. The native
iOS generated-project test compiles this path. Custom-component native-class
parameters resolve to their declared plugin class type and are borrowed, so a
child component cannot dispose its caller's instance. These source and
semantic changes advanced the cache schema to `build-v83`. Plugin native
component signatures are now available while custom component bodies are
lowered, so wrappers can compose qualified native views; the VideoPlayer
compile regression exercises this path on Swift and Kotlin. This compiler
change advances the cache schema to `build-v84`.

## Independent Swift/Kotlin review

The independent Luna review confirmed the same priorities and additionally
identified Cronet callback cancellation races, pinned-engine cache misses,
blocking download writes on the callback executor, and coarse Compose baseline
dependencies. The cancellation, cache, and callback-I/O findings are fixed
above; reducing the remaining Compose baseline without breaking generated API
stability remains follow-up work. `nexa audit --release-sizes` now builds
optimized native Release artifacts and reports their measured file and payload
sizes when the relevant toolchains are available. No zero-copy ABI claim is
made.

## Remaining architectural work

Add C++ event contracts on both platforms and broaden nested `Set`/`Map`
combinations. Android and iOS typed-error adapters translate supported
`NexaResult` failures into native exception/error cases, and async methods now
use the existing collection adapters on the background execution path.
Recursive `Array` adapters now have iOS Swift/C++ typecheck and Android
Kotlin/JNI runtime coverage, and flat-map adapters have per-platform type
coverage. Navigation gives each screen destination independent native state and
a unique Swift route identity; reusing native resources across separate route
lifetimes remains open. Direct Swift/Kotlin remains the default path.
