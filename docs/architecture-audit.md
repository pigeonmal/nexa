# Nexa architecture audit

Audit date: 2026-09-22

This audit was performed against the Rust workspace, the CLI project
scaffolder, both native backends, and representative generated projects for
minimal UI, lists, navigation, permissions, images, network calls, and plugins.
The initial pass was read-only. The second section records the priority fixes
implemented after the findings were reviewed.

## Findings before the fixes

### P0: plugin model and backend capability ownership

- Plugins were limited to a local `interfaces.nxid` plus native source trees.
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
- Added a versioned `abi.nxabi` contract to native plugin scaffolds. The
  contract validates C calling convention, buffer ownership, and lifetimes
  before binding generation and explicitly identifies call-scoped borrowed
  bytes as the only zero-copy-safe shape currently available.
- Added `nexa audit`, which emits a machine-readable report of optimized IR
  capabilities, generated source bytes, and target dependencies. Project
  generation now emits `nexa.sources.json`, an explicit list of generated
  source/resource units.
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

## Independent Swift/Kotlin review

The independent Luna review confirmed the same priorities and additionally
identified Cronet callback cancellation races, pinned-engine cache misses,
blocking download writes on the callback executor, and coarse Compose baseline
dependencies. The cancellation, cache, and callback-I/O findings are fixed
above; reducing the remaining Compose baseline without breaking generated API
stability remains follow-up work. The current patch does not claim a zero-copy
ABI or measured binary-size reduction without release builds.

## Remaining architectural work

1. Generate C++/Rust adapters from `abi.nxabi`. The schema and validation are
   now present, but adapter emission and native model layout are still pending.
2. Extend `nexa audit` with Android R8/resource-shrink results and Swift
   release binary/resource sizes when native release toolchains are available.
3. Define certificate pinning as one representation on both platforms (the
   current Swift leaf-certificate and Android public-key semantics differ).
