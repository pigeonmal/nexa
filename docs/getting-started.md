# Getting Started with Nexa 🚀

This guide walks you through setting up your environment, creating a new project, configuring app metadata, and mastering the live development workflow.

---

## 1. System Requirements

| Toolchain | Minimum Version | Required For | Verification Command |
|---|---|---|---|
| **Rust** | `1.85.0+` | Nexa compiler & CLI | `rustc --version` |
| **macOS & Xcode** | `macOS 14+`, `Xcode 15+` | iOS builds & simulators | `xcodebuild -version` |
| **Android SDK & JDK** | `JDK 17+`, `SDK 34+` | Android builds & emulators | `javac -version`, `adb --version` |

> [!TIP]
> Run `nexa doctor` at any time to verify that your native build toolchains, simulators, and environment variables are properly configured.

---

## 2. Installation

Install the Nexa CLI directly from the source repository:

```bash
git clone --recurse-submodules https://github.com/pigeonmal/nexa.git
cd nexa
cargo build --release -p nexa-cli
sudo cp target/release/nexa /usr/local/bin/
nexa --version
```

---

## 3. Project Creation & Structure

Create a new application with the standard scaffold:

```bash
nexa create Pulse
cd Pulse
```

### Directory Layout

```text
Pulse/
├── App.nx                   # Application UI, screens, and business logic
├── nexa.config.nx           # App identity, SDK levels, permissions, and dependencies
├── nexa.lock                # Deterministic native plugin lockfile
├── assets/                  # Shared cross-platform assets (images, fonts, icons)
│   ├── icon.png             # Default application launcher icon
│   └── splash.png           # Native launch screen image
└── build/                   # Ephemeral generated native projects (do not edit)
    ├── ios/                 # Xcode project (.xcodeproj) with SwiftUI host
    └── android/             # Gradle project with Jetpack Compose host
```

---

## 4. Configuration: `nexa.config.nx`

All application identifiers, orientations, native dependencies, and target platform floors are configured declaratively in `nexa.config.nx`:

```nexa
config {
    app {
        displayName: "Pulse App"
        version: "1.0.0"
        buildNumber: 1
        orientation: "portrait-phones"
    }

    ios {
        bundleIdentifier: "dev.nexa.pulse"
        minVersion: "16.0"
        arch: ["arm64"]
    }

    android {
        applicationId: "dev.nexa.pulse"
        minSdk: 23
        targetSdk: 36
        arch: ["arm64", "x86_64"]
        cronet {
            provider: "play-services"
            diskCacheSizeMb: 64
        }
    }

    dependencies {
        sqlite: "dev.nexa.sqlite"
        mmkv: "dev.nexa.mmkv"
    }
}
```

### Configuration Options Reference

| Section | Key | Type | Default | Description |
|---|---|---|---|---|
| `app` | `displayName` | `String` | Required | Human-readable app name shown on the device launcher. |
| `app` | `version` | `String` | `"1.0.0"` | Semantic version string (`MAJOR.MINOR.PATCH`). |
| `app` | `buildNumber` | `Int32` | `1` | Monotonically increasing build integer. |
| `app` | `orientation` | `String` | `"all"` | `"all"`, `"portrait"`, or `"portrait-phones"`. |
| `ios` | `bundleIdentifier`| `String` | Required | Reverse-DNS iOS App ID (e.g. `com.company.app`). |
| `ios` | `minVersion` | `String` | `"16.0"` | Minimum supported iOS deployment target. |
| `ios` | `arch` | `Array<String>`| `["arm64"]` | Target architectures (`"arm64"`, `"x86_64"`). |
| `android` | `applicationId`| `String` | Required | Android Package Name. |
| `android` | `minSdk` | `Int32` | `23` | Minimum Android API level (Android 6.0 Marshmallow). |
| `android` | `targetSdk` | `Int32` | `36` | Target Android SDK version. |
| `android` | `cronet.provider`| `String` | `"play-services"`| `"play-services"` or `"embedded"` network stack. |
| `dependencies`| `<alias>` | `String` | Optional | Plugin package ID to resolve from official or local registry. |

---

## 5. Development Workflow & CLI

```bash
# 1. Type check without compiling native platforms (super-fast)
nexa check

# 2. Start live development with hot reload
nexa dev

# 3. Target a specific platform
nexa dev --ios
nexa dev --android
```

### CLI Command Matrix

| Command | Key Flags | Description |
|---|---|---|
| `nexa create <name>` | `--template <name>` | Scaffolds a new project with idiomatic layout. |
| `nexa check` | `--locked`, `--audit` | Validates `.nx` source, plugin dependencies, and types. |
| `nexa dev` | `--ios`, `--android`, `--arch <arch>`, `--compile-only` | Builds and launches simulator with DevRuntime hot reload. |
| `nexa test` | `--unit-only` | Executes in-language `test` blocks and unit test suites. |
| `nexa release` | `--flavor <name>`, `--ios`, `--android` | Generates signed Android AABs and production iOS IPAs. |
| `nexa doctor` | N/A | Verifies toolchain installation and environment health. |
| `nexa audit <file>` | `--target <swift\|kotlin\|all>`, `--out <path>` | Analyzes binary footprint, reachable APIs, and warnings. |

### Interactive DevRuntime Shortcuts

While `nexa dev` is running in your terminal:

| Key Shortcut | Action | Description |
|---|---|---|
| `r` | **Hot Reload** | Updates views, functions, state initializers, and component trees instantly. |
| `Shift + R` | **Hot Restart** | Clears in-memory state and restarts the application flow. |
| `b` | **Rebuild Native Host**| Re-runs native compiler (Xcode/Gradle) when plugins or native code change. |
| `p` | **Toggle Overlay** | Toggles the on-device live FPS and frame latency diagnostics HUD. |

---

## 6. Your First Complete App

Here is a realistic counter and note app in `App.nx` demonstrating state, input, and list rendering:

```nexa
struct Note {
    id: Int64,
    text: String,
    timestamp: String,
}

app QuickNotes {
    state notes: Array<Note> = [
        Note(1, "Install Nexa CLI", "10:00 AM"),
        Note(2, "Build native iOS & Android app", "10:15 AM")
    ]
    state draftText: String = ""

    body {
        NavigationStack {
            Column(spacing: 16, padding: 16) {
                // Input controls
                Row(spacing: 8) {
                    TextInput(
                        value: draftText,
                        placeholder: "Type a quick note...",
                        onChange: text => { draftText = text }
                    )
                    Button("Add Note", icon: "plus") {
                        if draftText.trim() != "" {
                            let newId = (notes.last()?.id ?? 0) + 1
                            notes.push(Note(newId, draftText, "Just now"))
                            draftText = ""
                        }
                    }
                }

                // Render virtualized list
                if notes.isEmpty {
                    ContentUnavailable(
                        title: "No Notes Yet",
                        description: "Add your first note using the input above.",
                        icon: "note.text"
                    )
                } else {
                    FastList(notes, key: "id") { note in
                        Row(spacing: 12, padding: 12, background: "#1E293B", cornerRadius: 8) {
                            Icon(system: "note", size: 20, tint: "#38BDF8")
                            Column(spacing: 2) {
                                Text(note.text)
                                    .fontSize(16)
                                    .foregroundColor("#FFFFFF")
                                Text(note.timestamp)
                                    .fontSize(12)
                                    .foregroundColor("#94A3B8")
                            }
                            Spacer()
                            Button(icon: "trash") {
                                notes = notes.filter(n => n.id != note.id)
                            }
                        }
                    }
                }
            }
        }
    }
}
```

---

## 7. Next Steps

- Explore the complete [**Language Guide**](language-guide.md) to learn about static typing, collections, and `Result<T, E>`.
- Browse the full [**Component Catalog**](components.md) for layout, styling modifiers, and interactive controls.
- Check [**State & Navigation**](state-and-navigation.md) for multi-screen stacks, modal sheets, and reactive signals.
