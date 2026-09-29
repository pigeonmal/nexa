# Nexa Architecture & Performance Model

Nexa is engineered around a core tenet: **generated native code must match or outperform hand-written native code**.

---

## 1. Compilation Pipeline

```mermaid
graph TD
    NX[".nx Source Code"] --> LexerParser["nexa-syntax\n(Lexer & Recursive Descent Parser)"]
    LexerParser --> AST["Abstract Syntax Tree (AST)"]
    AST --> SemanticCompiler["nexa-compiler\n(Type Checker, Validator & Optimizer)"]
    SemanticCompiler --> IR["Typed Intermediate Representation (nexa-ir)"]
    
    IR --> SwiftBackend["nexa-backend-swift\n(Native SwiftUI Code Generator)"]
    IR --> KotlinBackend["nexa-backend-kotlin\n(Native Jetpack Compose Code Generator)"]
    
    PluginIDL["native.nxid"] --> Codegen["nexa-codegen\n(Swift/Kotlin/C++ Bridge Generator)"]
    Codegen --> SwiftBackend
    Codegen --> KotlinBackend
    
    SwiftBackend --> XcodeBuild["Native iOS App (Xcode / Swift 6)"]
    KotlinBackend --> GradleBuild["Native Android App (Gradle / Kotlin 2)"]
```

---

## 2. Performance Characteristics

### 1. Zero Runtime Reflection or Dynamic Boxing
Unlike cross-platform engines that use untyped hash maps or reflection (`Any`, `Object`, `Mirror`), Nexa statically types all variables, expressions, and parameters directly into concrete Swift and Kotlin types.

### 2. Elimination of `AnyView` in SwiftUI
Type-erased views such as `AnyView` hide a view's concrete structure from SwiftUI. Nexa's generated view hierarchy uses concrete view types and does not emit `AnyView` wrappers:
```swift
// Emitted by Nexa:
VStack(spacing: 16) {
    Text(count.description)
}
// NOT: AnyView(VStack { ... })
```

### 3. Unboxed Compose State in Kotlin
In Jetpack Compose, generic state holders represent numeric values as boxed values. Nexa selects primitive-specialized state holders for supported numeric types:
```kotlin
// Emitted by Nexa:
val count = remember { mutableIntStateOf(0) }
val progress = remember { mutableDoubleStateOf(0.0) }
```

### 4. Typed C++ Bridging
C++ native plugins use generated bindings and standard C++ types such as `std::int32_t` and `std::vector<std::uint8_t>`. The bindings avoid a serialization format, while converting collection values at language boundaries may copy their contents.

## 3. Generated Source Cache

`build-v120` decodes nullable generic Android plugin collection inputs before dispatch. `build-v119` makes Android launcher orientation follow the user's current orientation policy at startup.

The CLI fingerprints the source graph and a generator schema version before reusing generated native output. The schema is advanced when compiler or backend behavior changes without a source-graph change, so cached projects cannot retain stale generated code. `build-v118` adds typed Haptics calls to both native backends and DevRuntime dispatch. `build-v117` fixes recursive Kotlin DevRuntime decoding for optional generic plugin arguments. `build-v116` adds the typed iOS and Android system text clipboard API and DevRuntime dispatch. `build-v115` emits Swift and Kotlin read codecs for optional values nested inside compound generic plugin values, keeping Kotlin decode failures separate from valid null elements. `build-v114` includes SecureStorage native adapters in every DevRuntime host so hot reload can add the API without rebuilding. `build-v113` adds typed core cryptographic helpers to both native backends and DevRuntime. `build-v112` adds optional generic plugin input codecs and qualified service calls with explicit type arguments. `build-v111` adds locale-aware `Number.formatCurrency` to both AOT backends and DevRuntime. `build-v110` adds cross-platform generic plugin codecs for `Result` values with enum failures, including hot-reload value adapters. `build-v109` adds configurable spring response and damping to typed animation specs and updates the Dev IR format. `build-v105` reads Android Cronet provider and disk-cache settings from `nexa.config.nx`, omits the embedded artifact for the default Play Services provider, and advances the generated host schema. `build-v104` adds Pressable double-tap actions to typed IR, both native backends, and both Dev renderers, advancing the Dev IR format version. `build-v100` adds typed visual effects to the native style backends and both Dev renderers, advancing the Dev IR format version. `build-v99` adds a native menu-style Picker and Dev IR/runtime support on both platforms, advancing the Dev IR format and protocol versions. `build-v98` adds native SegmentedControl selection and Dev IR/runtime support on both platforms, advancing the Dev IR format and protocol versions. `build-v97` adds native Dialog alerts and Dev IR/runtime support on both platforms, advancing the Dev IR format and protocol versions. `build-v96` adds the native Float64 Slider and progress indicators and extends both development runtimes with their state/expression handling; the Dev IR format and protocol versions reject older runtimes. `build-v95` adds typed arithmetic and string concatenation plus native collection `count`/`isEmpty` accessors; both development interpreters understand the new expression nodes, and the Dev IR and protocol versions reject older runtimes. `build-v94` iterates Compose state collections through their zero-copy immutable backing snapshots, avoiding a snapshot-state read for every loop element. `build-v93` binds the Android runtime context before native plugin initialization and emits the corresponding context import/runtime helper only for modules with reachable plugins. `build-v92` included the development host's first-party File and Path helpers required when those APIs are added by hot reload.

## 4. Host Project Generation

Generating a native host project is split into three stages so that *what* gets
generated can be inspected and tested without writing a project.

```mermaid
graph LR
    IR["Typed IR"] --> Units["Source units\n(nexa-codegen)"]
    Pkgs["Plugin packages"] --> Prepare["prepare\n(copies & discovery)"]
    Units --> Prepare
    Prepare --> Plan["ios_plan / android_plan\n(computed, no I/O)"]
    Plan --> Validate["ProjectPlan::validate\n(names & paths)"]
    Validate --> Write["writers\n(materialize)"]
```

- **Units.** The backends emit named compile units, each carrying its own file
  name and body, rather than one string with marker comments. Assembling those
  bodies into files is a separate, explicit step, because a host project has to
  adjust the header first: plugin bindings add imports every file must see, and
  Kotlin needs a `package` line ahead of them.
- **Prepare.** The filesystem work whose results the Xcode project and Gradle
  scripts must reference: staging plugin sources, vendoring artifacts, copying
  images.
- **Plan.** Every path, name, and file body, computed from prepared facts with
  no filesystem access. Plans carry text files, binary files such as the Gradle
  wrapper jar, executable bits, and `CopyAction`s for artifacts vendored from a
  plugin package -- all as data.

Staging a plugin artifact follows the same split. `stage_ios_plugin_artifacts`
validates each XCFramework and returns the name the Xcode project references
plus a `CopyAction`; the plan turns the difference against the staging marker
into removals and writes the marker as an ordinary planned file. Only the
writer copies or deletes.
- **Validate.** One place decides whether names and paths are sound: duplicate
  unit names, a unit name that is really a path, a planned file colliding with a
  compile unit, a path escaping the project root, or a plan that both writes and
  removes the same file.
- **Write.** Content-addressed writes, so a rebuild does not churn timestamps or
  re-trigger downstream incremental builds.

Correctness is enforced by three byte-identity harnesses that render real output
and print a digest per artifact: `app_fingerprint` (generated Swift and Kotlin),
`bridge_fingerprint` (the plugin C++/Swift/JNI bridge), and
`project_fingerprint` (every file a generated host project contains).
