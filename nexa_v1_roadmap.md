# Nexa 1.0 Complete Architecture Roadmap & Execution Plan

## Goal Description

Nexa is an Ahead-Of-Time (AOT) transpiler that compiles declarative `.nx` mobile applications directly into native Swift (SwiftUI) for iOS and Kotlin (Jetpack Compose) for Android with zero runtime overhead, zero reflection, zero dynamic interpretation, and uncompromised native performance.

This document represents the **final, verified architectural specification and execution roadmap for Nexa 1.0**. Following a comprehensive gap audit across real-world mobile engineering requirements, this plan ensures Nexa contains every component, API, language primitive, and platform optimization needed to ship any production application—from a To-do list to TikTok, Spotify, and E-commerce platforms—without writing a single line of manual Swift or Kotlin.

---

## The 8 App Archetypes: Capability Matrix

This matrix verifies that Nexa 1.0 has zero missing dependencies for all 8 target application archetypes:

| App Archetype                  | Core Components & Layout                                                              | Core APIs Used                                                                   | Official Plugin Required                                       |
| ------------------------------ | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **1. To-Do App**               | `Column`, `Row`, `FastList`, `TextInput`, `Button`, `Switch`, `Badge`, `Divider`      | `Storage`, `SecureStorage`, `Time`                                               | None (100% Core)                                               |
| **2. E-Commerce Store**        | `FastList(Grid)`, `Image(url:)`, `BottomSheet`, `Badge`, `Slider`, `SegmentedControl` | `Network.fetch`, `Json.parse`, `Number.formatCurrency`, `MediaPicker`            | None (100% Core)                                               |
| **3. Chat App**                | `FastList` (reverse anchor), `TextInput` (send), `Avatar`, `Badge`                    | `Network.isOnline`, `Clipboard`, `Haptics`, `SecureStorage`                      | `@nexa/websocket` (long-lived real-time connection)             |
| **4. Weather Dashboard**       | `RefreshControl`, `Stack`, `ProgressRing`, `Text`, `Spacer`                           | `Permissions.request(Location)`, `Network.fetch`, `Json.parse`, `BackgroundTask` | None (100% Core)                                               |
| **5. Spotify-Style Music App** | `FastList` (virtualized tracks), `Slider` (scrubber), `BottomSheet`, `.sharedElement` | `Storage`, `Haptics`, `AudioSession`                                             | `@nexa/audio-player` (Background media, lock screen)           |
| **6. Netflix-Style Video App** | `FastList(Horizontal)` inside `FastList(Vertical)`, `Stack`, `Spacer`                 | `Screen.lockOrientation(Landscape)`, `Network.fetch`, `Json.parse`               | `@nexa/video-player` (HLS/DASH streaming, video surface)       |
| **7. Fitness Tracker**         | `ProgressRing` (activity rings), `FastList` (workout history), `SegmentedControl`     | `Time`, `BackgroundTask` (sync), `SecureStorage`                                 | `@nexa/sensors` (Pedometer, accelerometer) + `@nexa/mmkv`      |
| **8. TikTok-Style Video App**  | `FastList(Vertical)` (paging snap), `Pressable` (.onDoubleTap), `Stack`, `Avatar`     | `Haptics.impact`, `Network.fetch`, `MediaPicker`                                 | `@nexa/video-player` (Vertical snap player with pre-buffering) |

---

## Detailed Architectural Blueprint

```mermaid
graph TD
    subgraph Frontend["Language, Parser & Compiler"]
        Syntax["Syntax & Parser<br/>(Arithmetic + - * / %, Infix String +, ? Propagation, Closures, Tests)"]
        Semantics["Semantic Analyzer & Type System<br/>(Result&lt;T,E&gt;, Optionals, Generics, Enums, Structs)"]
        IR["Typed Intermediate Representation<br/>(Nodes, Expressions, Animations, Gestures)"]
    end

    subgraph CoreLibraries["Core Standard Library (Built into Nexa)"]
        NetComms["HTTP Network & Connectivity<br/>(URLSession / Cronet, NWPathMonitor)"]
        Security["SecureStorage & Crypto<br/>(Keychain / Keystore, CryptoKit / MessageDigest)"]
        MediaHardware["MediaPicker, Screen & Haptics<br/>(PHPicker / PhotoPicker, Orientation, Vibrator)"]
        FormatUtils["Time, Currency, Json, Random<br/>(NumberFormatter, Codable / Serializer)"]
        SystemServices["Clipboard, BackgroundTask, Storage<br/>(UIPasteboard, BGTaskScheduler / WorkManager)"]
    end

    subgraph NativeBackends["Native AOT Generators"]
        SwiftBackend["SwiftUI Generator (iOS 17+)<br/>(@Observable, EquatableView, Task, PrivacyInfo)"]
        KotlinBackend["Jetpack Compose Generator (Android)<br/>(Compose Stability, Cronet Play Services, R8)"]
    end

    subgraph Verification["Decoupled Native Verification Skills"]
        SwiftExpert["nexa-swift-expert<br/>(Audit @Observable, Zero AnyView, Memory Retain Cycles)"]
        KotlinExpert["nexa-kotlin-expert<br/>(Audit Compose Stability, Recomposition Phase, State Unboxing)"]
    end

    subgraph OfficialPlugins["Official nexa-plugins Ecosystem (Standalone Repo)"]
        Video["@nexa/video-player (AVPlayer / Media3 ExoPlayer)"]
        Audio["@nexa/audio-player (MediaSession / NowPlaying)"]
        MMKV["@nexa/mmkv (High-performance mmap KV)"]
        Camera["@nexa/camera (CameraX / AVCaptureSession)"]
        Maps["@nexa/maps (MapKit / Google Maps)"]
        SQLite["@nexa/sqlite (Relational database with migrations)"]
        Biometrics["@nexa/biometrics (Face ID / BiometricPrompt)"]
        WebView["@nexa/webview (WKWebView / WebView)"]
        Sensors["@nexa/sensors (CoreMotion / SensorManager)"]
        IAP["@nexa/in-app-purchases (StoreKit 2 / Google Play Billing)"]
    end

    Syntax --> Semantics --> IR
    IR --> CoreLibraries
    CoreLibraries --> SwiftBackend
    CoreLibraries --> KotlinBackend
    SwiftBackend -. "Audited by" .-> SwiftExpert
    KotlinBackend -. "Audited by" .-> KotlinExpert
    OfficialPlugins -. "Bound via .nxid" .-> IR
```

---

## Phased Implementation Roadmap

### Phase 1: Language Syntax, Arithmetic & Error Handling Completeness

#### 1.1 Infix Arithmetic & String Operations

- Update `crates/nexa-syntax` lexer and parser to support binary arithmetic:
  - Additive: `+` (numeric addition and string concatenation), `-`
  - Multiplicative: `*`, `/`, `%`
  - Compound assignment: `+=`, `-=`, `*=`, `/=`
  - Unary negation: `-expr`, `!expr`
- basic methods for primitive types (string, number, lists, etc)
- Implement strict operator precedence in `crates/nexa-syntax/src/parser.rs`.
- Update constant folding and dead code elimination in `crates/nexa-compiler/src/optimize.rs`.
- Lower arithmetic directly to native Swift and Kotlin operators in `nexa-backend-swift` and `nexa-backend-kotlin`.

#### 1.2 First-Class Error Handling (`Result<T, E>` & `?`)

- Full syntax support for `Result<T, E>`, `Ok(val)`, and `Err(err)` constructors.
- Postfix `?` propagation operator:
  - Functions returning `Result<T, E>` early-return on `Err`.
  - Swift lowering: `Result<T, E>` with `.get()` or native `throws`.
  - Kotlin lowering: Zero-allocation sealed `NexaResult<T, E>`.
- Typed `try { ... } catch (e: CustomError) { ... } else { ... }` blocks in actions and async functions.

#### 1.3 Developer Ergonomics Polish

- [x] Import local `.nx` modules recursively, including standalone screens and reusable tab components; hot reload picks up added or edited imported files.
- Inline conditional expressions: `let val = if condition { a } else { b }` and `condition ? a : b`.
- Clean modifier chaining: `Text("Hi").fontSize(18).bold().padding(12)`.
- Collection utilities: `array.random()`, `array.shuffled()`, `array.reverse()`, `array.slice(start..end)`, `array.count`, `array.isEmpty`.

---

### Phase 2: Missing Core Components & Visual Modifiers

#### 2.1 Essential UI Components

- **`Spacer()`**: Expands along parent axis (SwiftUI `Spacer()`, Compose `Spacer(Modifier.weight(1f))`).
- **`Divider(color: String, thickness: Float64)`**: Horizontal / vertical separation lines.
- **`Slider(value: binding, min: Float64, max: Float64, step: Float64)`**: Scrubber control for music, volume, progress.
- **`ProgressBar(progress: Float64)` & `ProgressRing(progress: Float64)`** and ProgressIndicator: Linear and circular indicators for fitness/downloads.
- **`Dialog(isPresented: binding, title: String, message: String) { actions }`**: Modal alert prompts.
- **`SegmentedControl(items: Array<String>, selected: binding)`**: Tabbed filter bar.
  Picker

#### 2.2 Visual Modifiers (in style, maybe like opacity: .3)

- `.opacity(Float64)`, `.scale(Float64)`, `.rotation(Float64)`.
- `.shadow(radius: Float64, x: Float64, y: Float64, color: String)`.
- `.blur(radius: Float64)`.
- `.clip(shape: Rounded(Float64))`.
- `.zIndex(Int32)`.

#### 2.3 Hot reload

- [x] Detect edits and newly created or imported `.nx` modules, compile them to DevRuntime IR, and update the running app without copying `.nx` files into its native bundle.
- [x] Prelink configured plugin packages into the DevRuntime host. Existing plugin methods, typed errors, property writes, event subscriptions, and native component adapters can be used by reloaded modules for bridge-supported types.
- [x] Require a native rebuild when the host changes, including adding a plugin dependency or changing native plugin sources, plugin contracts, permissions, or platform minimums.
- [ ] Close the remaining partial DevRuntime coverage for plugin values, native component calls, property assignment, and event subscription. The log-only device probe in `scripts/test-dev-runtime-plugin-hot-reload.sh` now exercises class construction/methods, scalar and optional-array generic codecs, compound property writes, typed error payloads, class/component events, and a newly created imported component on both platforms. Expand its cases to the remaining supported IDL shapes and unsupported boundaries, then replace structural dispatch checks before marking this area complete.

#### 2.4 Accesibility

Move accessibility from a component to just a param inside other component like Image, text, etc

#### 2.5 Thread

I want be able to do tasks in Task or couroutines or custom thread

---

### Phase 3: Animations, Gestures & Transitions

#### 3.1 Custom & Spring Animations (very optimized animation for 120+ fps)

- Spring physics configuration: `Spring(response: Float64, damping: Float64)`.
- Explicit animation execution: `withAnimation(Spring(damping: 0.8)) { state = newValue }`.
- View transitions on conditional blocks: `.transition(.fade)`, `.transition(.slide(from: .bottom))`, `.transition(.scale)`.

#### 3.2 Shared Element Hero Transitions

- [x] Syntax: `Image(url: item.imageUrl).sharedElement(id: "item-42")`.
- [x] Swift lowering: SwiftUI `.matchedGeometryEffect(id:in:)`.
- [x] Kotlin lowering: Jetpack Compose `SharedTransitionLayout` + `Modifier.sharedElement`.
- [x] DevRuntime renders shared IDs after hot reload on iOS and Android.

#### 3.3 Gesture Handling Suite (very optimized gesture, for 120+ fps)

- Modifiers for interactive nodes:
  - [x] `.onTap { ... }` (legacy `.onPress` remains accepted)
  - [x] `.onDoubleTap { ... }` (TikTok double tap to like)
  - [x] `.onLongPress(durationMs: Int32) { ... }`
  - [x] `.onDrag { translationX, translationY, velocityX, velocityY -> ... }` (translation uses points/dp; velocity uses points/dp per second)
  - [x] `.onPinch { scaleFactor -> ... }` (Product photo zoom; multiplicative scale delta per gesture update)

---

### Phase 4: Core Standard APIs (99% Application Suite)

#### 4.1 Android Cronet Configuration

- Support in `nexa.config.nx`:
  ```nexa
  config {
      android {
          cronet {
              provider: "play-services" // or "embedded" (default: "play-services")
              diskCacheSizeMb: 64
          }
      }
  }
  ```
- Exclude `org.chromium.net:cronet-embedded` by default from Gradle dependencies, reducing Android APK size by ~10MB.

#### 4.2 Real-Time WebSockets & Network Connectivity

- [x] **`@nexa/websocket` plugin**:

  ```nexa
  plugin "dev.nexa.websocket" as WebSocket
  app Chat {
      let socket = WebSocket.WebSocket("wss://chat.example.com/ws")
      state received = ""
      body {
          OnAppear async {
              socket.messageReceived { message -> received = message }
              try {
                  await socket.connect()
              } catch {
                  case WebSocket.WebSocketError.invalidUrl { received = "Invalid URL" }
                  case WebSocket.WebSocketError.alreadyConnected { received = "Already connected" }
              }
          }
          OnDisappear { socket.dispose() }
          Button("Send") { socket.send("hello") }
          Text(received)
      }
  }
  ```

  - Swift: `URLSessionWebSocketTask` (callback APIs, iOS 13+).
  - Android: OkHttp WebSocket client 5.5.0 (Apache-2.0), isolated to this plugin. Cronet remains the HTTP transport; its request streams do not expose a WebSocket upgrade API.
  - Keep each socket's callbacks, cancellation, and close behavior scoped to its plugin instance. Reuse the shared OkHttp client and connection pool. Do not add a custom C++/JNI WebSocket framing layer.
  - `connect()` starts the handshake; observe state and failure events for its result. `send` reports local queue acceptance, not peer delivery. Dispose the socket from the owning screen's `OnDisappear`.

- **`Network.isOnline: Bool`** and **`Network.onStatusChange { isOnline in ... }`**:
  - Swift: `NWPathMonitor`.
  - Kotlin: `ConnectivityManager.NetworkCallback`.
- [x] **Multipart Upload**: `Network.upload(url: String, file: String, fields: Map<String, String>)` streams the file with native transport and bounds request-body memory.

#### 4.3 Security: `SecureStorage` & `Crypto`

- **`SecureStorage`**: Hardware-backed credential and auth token persistence.
  - iOS: Apple **Keychain Services** (`kSecClassGenericPassword`).
  - Android: **Android Keystore System** + `EncryptedSharedPreferences`.
- **`Crypto`**:
  - `Crypto.sha256(text) -> String`
  - `Crypto.sha512(text) -> String`
  - `Crypto.hmacSha256(key, message) -> String`
  - `Crypto.randomBytes(count) -> String`

#### 4.4 Media Picker & Screen Orientation

- **`MediaPicker`**: (media picker create a plugin for it)
  - `MediaPicker.pickImage() -> Optional<String>` (returns local file URI).
  - `MediaPicker.pickVideo() -> Optional<String>`.
  - Zero permissions required (uses iOS `PHPickerViewController` and Android Photo Picker).
- **`Screen.lockOrientation(mode)`**:
  - Canonical Nexa enums: `Portrait`, `Landscape`, `All`. Essential for video apps.
  - [x] Fix Android startup orientation: generated launcher activities now follow the user's current orientation policy, including landscape, from first launch.

#### 4.5 Formatting & Utilities

- **`Number.formatCurrency(amount: Float64, currencyCode: String) -> String`**: Locale-aware currency formatting.
- **`Json.parse<T>(raw: String) -> Result<T, JsonError>`** and **`Json.stringify(value: T) -> String`**: Zero-reflection parsing.
- [x] **`Storage`**: App-private string preferences with `getString`, `setString`, `delete`, and `clear` on both native platforms and DevRuntime.
- [x] **`Clipboard`**: `setText`, `getText`, and `hasText` on both native platforms and DevRuntime.
- [x] **`Haptics`**: `.impact(Light|Medium|Heavy)`, `.notification(Success|Error)`, `.selection()` on both native backends and DevRuntime.
- **`BackgroundTask`**: Periodic background execution (iOS `BGTaskScheduler`, Android `WorkManager`).

#### 4.6 Keyboard Ergonomics

- `Keyboard.dismiss()`
- `TextInput` parameters: `keyboardType: email | number | phone | url`, `isSecure: Bool` (passwords), `autofill: username | password | oneTimeCode`, `returnKeyType: done | search | send | next`.

#### 4.7 Automated Apple Privacy Manifest

- [x] Emit `PrivacyInfo.xcprivacy` for reachable required-reason APIs: app-private `UserDefaults` (`CA92.1`) and app-container file metadata (`C617.1`). SecureStorage's Keychain calls do not map to a required-reason API category and therefore do not add a fabricated reason entry.

---

### Phase 5: CLI Consolidation & In-Language Testing

#### 5.1 CLI Command Clarification

- `nexa doctor`: Diagnoses host development environment (Xcode, Swift, Android SDK, JDK, Gradle, adb).
- `nexa check`: Primary static verification gate (.nx syntax, semantic analysis, type checking, plugin contracts) with `--audit` flag for dead-code inspection.
- `nexa test`: Executes `.nx` unit tests and headless component tests.
- `nexa audit`: Release footprint profiler (`--release-sizes`), measuring exact `.app` and `.apk`/`.aab` sizes.

#### 5.2 In-Language Test Runner (`nexa test`)

- Syntax for test blocks in `.nx`:
  ```nexa
  test "cart item calculation" {
      let result = calculateTotal(price: 50.0, qty: 2)
      assert(result == 100.0)
  }
  ```
- [x] Parse named top-level `test` blocks with immutable `let` bindings and `assert(condition[, message])` statements.
- [x] Type-check test values with the compiler's existing function signatures and lower assertions to typed IR.
- [x] Evaluate deterministic synchronous test expressions through pure app-local functions in the host CLI; `nexa test --unit-only` skips native toolchain builds.
- Platform APIs, plugin calls, and async functions remain outside the host evaluator; native test-host compilation still runs after in-language tests under ordinary `nexa test`.

---

### Phase 6: Decoupled Native Quality AI Skills

We establish two specialized platform AI skills under `.agents/skills/`:

#### [NEW] `.agents/skills/nexa-swift-expert/SKILL.md`

- **Auditing Invariants**:
  Create an independant export skill for swift, with all the rules you already know for best practicies and best performance for swift ios app.

#### [NEW] `.agents/skills/nexa-kotlin-expert/SKILL.md`

- **Auditing Invariants**:
  Create an independant export skill for kotlin, with all the rules you already know for best practicies and best performance for kotlin android app.

---

### Phase 7: Official `nexa-plugins` Repository

Create the standalone `nexa-plugins` repository containing official native plugin packages:

1. [x] `@nexa/video-player`: AVPlayer (iOS) & Media3 ExoPlayer 1.11.1 (Android). Adaptive HLS/DASH, PiP, playback controls, embedded Cronet networking, and an optional LGPL-only FFmpeg fallback controlled by `VideoView.softwareDecodingEnabled`.
2. [x] `@nexa/audio-player`: Background audio playback, lock screen metadata (`MPNowPlayingInfoCenter`, `MediaSession`).
3. [x] `@nexa/mmkv`: Tencent MMKV zero-copy memory-mapped storage. The Android arm64 Release conformance benchmark records 1,000 identical string writes in 0.170–0.288 ms with compare-before-set enabled and 1,000 reads in 0.158–0.182 ms; comparison-disabled writes are retained as a diagnostic.
4. `@nexa/camera`: Photo/video capture, QR & barcode scanning (`AVCaptureSession` / `CameraX`). (zero copy stream for image ia)
5. `@nexa/maps`: Apple Maps (`MapKit`) & Google Maps SDK for Android.
6. `@nexa/sqlite`: Relational SQLite database with migrations and transactions. (ultra performance)
7. `@nexa/biometrics`: Face ID, Touch ID, and Android BiometricPrompt.
8. [x] `@nexa/webview`: Embedded `WKWebView` and Android `WebView` with HTTPS-only navigation and origin-gated two-way string messaging (`plugins/webview`).
9. `@nexa/notifications`: Local notifications & APNs/FCM push notifications.
10. [x] `@nexa/sensors`: Accelerometer, gyroscope, and pedometer on iOS and Android, with typed readings/errors, runtime motion permission handling, and explicit stream disposal.
11. `@nexa/in-app-purchases`: Apple StoreKit 2 & Google Play Billing 7.

---

## Verification Plan

### Automated Verification

```bash
# 1. Type checking across all targets
cargo check --workspace --all-targets

# 2. Strict linter verification
cargo clippy --workspace --all-targets

# 3. Comprehensive test suite
cargo test --workspace

# 4. Multi-thread concurrency collision gate
cargo test -p nexa-testkit -- --ignored
```

### Native Skill Audits

- Run `nexa-swift-expert` to audit generated SwiftUI code across all 8 example apps.
- Run `nexa-kotlin-expert` to audit generated Jetpack Compose code for recomposition skippability and stability metrics.

### End-to-End Application Builds

- Verify that all 8 apps (To-Do, E-Commerce, Chat, Weather, Spotify, Netflix, Fitness, TikTok) compile, generate valid Xcode and Gradle projects, and run smoothly at 60/120 FPS on iOS Simulators and Android Emulators.
- Run `scripts/test-dev-runtime-plugin-hot-reload.sh ios` and `scripts/test-dev-runtime-plugin-hot-reload.sh android` with a booted simulator/emulator to verify the prelinked plugin call path and late-created imported `.nx` component through native logs, without screenshots.
