# Nexa ⚡

**The Ahead-Of-Time (AOT) Native Mobile Framework**  
Compiles declarative `.nx` apps directly into 100% native **Swift (SwiftUI)** for iOS and **Kotlin (Jetpack Compose)** for Android.

[![License](https://img.shields.io/badge/license-MPL--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg)](https://www.rust-lang.org)
[![Swift](https://img.shields.io/badge/swift-6.0-red.svg)](https://swift.org)
[![Kotlin](https://img.shields.io/badge/kotlin-2.4-purple.svg)](https://kotlinlang.org)
[![Platforms](https://img.shields.io/badge/platforms-iOS%2016+%20%7C%20Android%2023+-brightgreen.svg)]()

---

## Why Nexa?

Nexa eliminates the performance and architectural compromises of legacy cross-platform tools. There is **no bundled JavaScript engine**, **no custom canvas renderer**, and **zero runtime reflection**.

| Architectural Dimension | React Native | Flutter | Kotlin Multiplatform | Nexa ⚡ |
|---|---|---|---|---|
| **UI Rendering Engine** | JS Bridge / Fabric | Impeller (C++ Canvas) | Compose Multiplatform | **100% Native SwiftUI & Jetpack Compose** |
| **Runtime Engine Footprint** | Bundled Hermes / V8 (~15 MB) | Bundled Flutter Engine (~10 MB) | Bundled JVM / Native Runtime | **Zero Overhead (Direct OS Frameworks)** |
| **UI Type Erasure** | N/A (Dynamic JS) | Skia / Impeller Objects | Composable State Boxing | **Zero `AnyView` / Primitive Unboxed States** |
| **Hot Reload & Dev DX** | Fast Refresh (JS only) | Hot Reload (Dart VM) | Experimental | **State-Preserving DevRuntime Hot Reload** |
| **Native Plugin Bridge** | JNI / TurboModules | MethodChannels / FFI | Expect/Actual / C-Interop | **Statically-Typed `.nxid` (Swift, Kotlin, C++20)** |
| **Reactive Database** | Community Async Bridges | SQLite FFI Wrappers | SQLDelight / Room3 | **Native `Signal<T>` with Cross-Process Invalidation** |

---

## Architecture Pipeline

```mermaid
graph TD
    NX[".nx Source Code"] --> Syntax["nexa-syntax\n(Lexer, Parser & AST)"]
    Syntax --> Compiler["nexa-compiler\n(Semantic Checks, Type Inference & Inlining)"]
    Compiler --> IR["nexa-ir\n(Typed Platform-Neutral IR)"]
    IR --> SwiftBackend["nexa-backend-swift\n(Swift 6 & SwiftUI Codegen)"]
    IR --> KotlinBackend["nexa-backend-kotlin\n(Kotlin & Jetpack Compose Codegen)"]
    PluginIDL[".nxid Plugin Contracts"] --> Codegen["nexa-codegen\n(Swift / Kotlin / C++20 JNI)"]
    Codegen --> SwiftBackend
    Codegen --> KotlinBackend
    SwiftBackend --> iOSBuild["Native iOS App (Xcode / SwiftPM)"]
    KotlinBackend --> AndroidBuild["Native Android App (Gradle / AGP)"]
    Syntax --> LSP["nexa-lsp (IDE Diagnostics & Autocomplete)"]
```

---

## 10-Second Quick Start

### 1. Installation

```bash
git clone --recurse-submodules https://github.com/pigeonmal/nexa.git
cd nexa
cargo build --release -p nexa-cli
sudo cp target/release/nexa /usr/local/bin/
```

### 2. Create & Launch an App

```bash
nexa create TaskFlow
cd TaskFlow
nexa dev
```

### CLI Command Reference

| Command | Syntax | Description |
|---|---|---|
| **Create Project** | `nexa create <name>` | Scaffolds a production-ready `.nx` app with Android & iOS templates. |
| **Type Check** | `nexa check` | Blazing-fast whole-project verification without launching native compilers. |
| **Live Development** | `nexa dev [--ios \| --android]` | Boots simulator/emulator with live DevRuntime hot reload. |
| **Run Tests** | `nexa test [--unit-only]` | Executes all in-language `.nx` `test` blocks and native test runners. |
| **Release Build** | `nexa release` | Emits signed Android AABs and iOS production IPA archives. |
| **Health Check** | `nexa doctor` | Verifies local Xcode, Android SDK, Rust, and Clang toolchains. |
| **Plugin Management**| `nexa plugin <add\|check\|generate>` | Validates and generates native `.nxid` plugin bindings. |

### Hot Reload Interactive Keys

When running `nexa dev`:
- `r` — Instantly reloads layout, functions, custom components, and styles.
- `Shift+R` — Hot restart (resets in-memory app state and reloads).
- `b` — Triggers a full native background rebuild and relaunch.

---

## Production Code Example

A real-world task card with priority tags, formatted timestamps, and reactive updates:

```nexa
struct TaskItem {
    id: Int64,
    title: String,
    priority: String,
    isCompleted: Bool,
    dueDate: String,
}

app TaskManager {
    state tasks: Array<TaskItem> = [
        TaskItem(1, "Audit security policies", "High", false, "Today, 5:00 PM"),
        TaskItem(2, "Review PR #412", "Medium", true, "Yesterday")
    ]
    state filterCompleted: Bool = false

    body {
        NavigationStack {
            Column(spacing: 16, padding: 16) {
                // Header Metrics Card
                Row(spacing: 12, padding: 16, background: "#1E293B", cornerRadius: 12) {
                    Icon(system: "checklist", size: 24, tint: "#38BDF8")
                    Column(spacing: 4) {
                        Text("Active Tasks")
                            .fontSize(14)
                            .foregroundColor("#94A3B8")
                        Text(tasks.filter(t => !t.isCompleted).count)
                            .fontSize(22)
                            .bold()
                            .foregroundColor("#FFFFFF")
                    }
                    Spacer()
                    Switch(value: filterCompleted, label: "Hide Done")
                }

                // Interactive Task List
                FastList(tasks.filter(t => !filterCompleted || !t.isCompleted), key: "id") { task in
                    Row(spacing: 12, padding: 12, background: "#0F172A", cornerRadius: 8) {
                        Button(icon: task.isCompleted ? "checkmark.circle.fill" : "circle") {
                            task.isCompleted = !task.isCompleted
                        }
                        Column(spacing: 4) {
                            Text(task.title)
                                .fontSize(16)
                                .bold()
                                .foregroundColor(task.isCompleted ? "#64748B" : "#F8FAFC")
                            Text(task.dueDate)
                                .fontSize(12)
                                .foregroundColor("#64748B")
                        }
                        Spacer()
                        Text(task.priority)
                            .fontSize(12)
                            .padding(horizontal: 8, vertical: 4)
                            .background(task.priority == "High" ? "#EF4444" : "#3B82F6")
                            .cornerRadius(4)
                            .foregroundColor("#FFFFFF")
                    }
                }
            }
        }
    }
}
```

---

## Core Framework Guides

| Guide | Description | Key Topics |
|---|---|---|
| 🚀 [**Getting Started**](docs/getting-started.md) | Setup, scaffolding, and CLI workflow | Toolchains, `nexa.config.nx`, targets, flavors |
| 📖 [**Language Guide**](docs/language-guide.md) | Full language specification | Types, `Result<T, E>`, collections, async/await |
| 🧩 [**Component Catalog**](docs/components.md) | Complete 100% UI Reference | All 45 components, parameter tables, modifiers |
| 🧭 [**State & Navigation**](docs/state-and-navigation.md) | Navigation and reactivity | `NavigationStack`, `AppBottomBar`, `Signal<T>` |
| 🔌 [**Native Plugins**](docs/plugins.md) | Native platform extension architecture | `.nxid` IDL, Swift, Kotlin, C++20 JNI |
| 📱 [**Native Widgets**](docs/widgets.md) | iOS WidgetKit and Android Glance | Timelines, refresh policies, shared storage |
| 🎨 [**System Icons**](docs/system-icons.md) | Unified cross-platform icon catalog | SF Symbols and Material Symbols mapping |
| 🏗️ [**Architecture & Performance**](docs/architecture.md) | Compiler internals & benchmarks | Unboxed primitives, memory safety, codegen |

---

## Official Native Plugins

Nexa maintains a first-party native plugin ecosystem under [`plugins/`](https://github.com/pigeonmal/nexa-plugins):

| Plugin | Package ID | Key Capabilities | Supported Platforms |
|---|---|---|---|
| **SQLite** | `dev.nexa.sqlite` | Reactive `observeQuery<T>`, WAL mode, cross-process sync | iOS 13+ \| Android 23+ |
| **MMKV** | `dev.nexa.mmkv` | High-speed memory-mapped key-value storage | iOS 12+ \| Android 21+ |
| **Notifications** | `dev.nexa.notifications` | Scheduled local notifications, badge counts, actions | iOS 13+ \| Android 23+ |
| **Camera** | `dev.nexa.camera` | Native camera preview, photo capture, video recording | iOS 14+ \| Android 23+ |
| **Media Picker** | `dev.nexa.media-picker` | System photo and video picker without privacy permissions | iOS 14+ \| Android 23+ |
| **Audio Player** | `dev.nexa.audio-player` | Background audio streaming, lock screen playback controls | iOS 14+ \| Android 23+ |
| **Video Player** | `dev.nexa.video-player` | Hardware-accelerated HLS and MP4 video playback | iOS 14+ \| Android 23+ |
| **Biometrics** | `dev.nexa.biometrics` | Face ID, Touch ID, and Android BiometricPrompt | iOS 13+ \| Android 23+ |
| **Maps** | `dev.nexa.maps` | Interactive MapKit & Google Maps views with pins | iOS 14+ \| Android 23+ |
| **Webview** | `dev.nexa.webview` | In-app browser engine with two-way JavaScript bridge | iOS 14+ \| Android 23+ |
| **Websocket** | `dev.nexa.websocket` | Low-latency binary and text WebSockets with auto-reconnect | iOS 13+ \| Android 23+ |
| **Sensors** | `dev.nexa.sensors` | Accelerometer, gyroscope, and magnetometer telemetry | iOS 13+ \| Android 23+ |
| **In-App Purchases** | `dev.nexa.in-app-purchases` | StoreKit 2 and Google Play Billing subscriptions | iOS 15+ \| Android 24+ |
| **Browser** | `dev.nexa.browser` | In-app Safari and Chrome Custom Tabs for OAuth flows | iOS 13+ \| Android 23+ |
| **Data Extractor** | `dev.nexa.data-extractor` | On-device Vision OCR, barcode scanning, text recognition | iOS 14+ \| Android 23+ |
| **Mail Composer** | `dev.nexa.mail-composer` | Native email composition sheets with attachments | iOS 13+ \| Android 23+ |

---

## Workspace Verification

To build and verify the compiler suite locally:

```bash
export CARGO_TARGET_DIR=/tmp/nexa-verification-target

# 1. Type checking across all 15 crates
cargo check --workspace --all-targets

# 2. Strict linter verification
cargo clippy --workspace --all-targets

# 3. Complete test suite
cargo test --workspace
```

---

## License

Nexa is open-source software licensed under the **Mozilla Public License 2.0 (MPL-2.0)**. See [LICENSE](LICENSE) for details.
