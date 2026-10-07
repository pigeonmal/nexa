# Getting Started with Nexa 🚀

This guide walks you through setting up your environment, creating a new project, configuring app metadata, and mastering the live development workflow.

| **Scope**: install, create, configure, and run | **Targets**: iOS and Android | **Prerequisite**: Rust 1.85+ |

## Quick start

Put this in `App.nx` in a Nexa project and run `nexa check`:

```nx
app ReadingQueue {
    state booksRead: Int32 = 4

    body {
        Column(spacing: 12, padding: 16) {
            Text("Books read: \(booksRead)", fontSize: 22, fontWeight: Bold)
            Button("Finish a book") {
                booksRead = booksRead + 1
            }
        }
    }
}
```

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
├── App.nx                   # Application UI, state, and behavior
├── nexa.config.nx           # App identity, platform floors, and permissions
├── .nexa/                   # Local signing settings; ignored by Git
└── build/                   # Native output created by dev, test, or release
```

---

## 4. Configuration: `nexa.config.nx`

App identity, platform floors, network settings, and permission purpose messages live in `nexa.config.nx`. `nexa create` generates this configuration shape:

```nexa
config {
    app {
        displayName: "Pulse App",
        version: "1.0.0",
        buildNumber: 1,
        orientation: "portrait-phones",
        deepLinks: []
    }
    flavors { staging { suffix: "staging" } }
    assets { icon: "", splash: "" }
    ios {
        minVersion: "16.0",
        bundleIdentifier: "dev.nexa.pulse",
        arch: ["arm64"],
    }
    android {
        minSdk: 23,
        targetSdk: 36,
        applicationId: "dev.nexa.pulse",
        arch: ["arm64", "x86_64"],
        cronet { provider: "play-services", diskCacheSizeMb: 64 },
    }
    permissions {
        camera: "Scan a return label to look up an order.",
        photos: "Choose a profile image for your account."
    }
}
```

### Configuration Options Reference

> Generated from `ProjectConfig::from_defaults`, so the Default column is what the compiler
> substitutes for an omitted key. Run `cargo test -p nexa-cli --test project_config_reference`
> with `NEXA_UPDATE_SNAPSHOTS=1` to regenerate.

<!-- nexadoc:begin config-options -->
| Section | Key | Type | Default | Description |
|---|---|---|---|---|
| `app` | `displayName` | `String` | `"NexaApp"` | Human-readable app name shown on the device launcher. |
| `app` | `version` | `String` | `"1.0.0"` | Semantic version string (`MAJOR.MINOR.PATCH`). |
| `app` | `buildNumber` | `Int32` | `1` | Monotonically increasing build integer. |
| `app` | `orientation` | `String` | `"all"` | `"all"`, `"portrait"`, or `"portrait-phones"`. |
| `app` | `deepLinks` | `Array<String>` | `[]` | App URL schemes and domains used by native project metadata. Incoming Nexa screen routing is not implemented. |
| `app` | `stagingSuffix` | `String` | `"staging"` | Application ID suffix applied by the implicit `staging` flavor. |
| `assets` | `icon` | `String` | `—` | Path to the app icon asset. |
| `assets` | `splash` | `String` | `—` | Path to the splash image asset. |
| `flavors.<name>` | `suffix` | `String` | `—` | Application ID suffix for that named flavor. Defaults to the flavor name when omitted. |
| `ios` | `minVersion` | `String` | `"16.0"` | Minimum supported iOS deployment target. |
| `ios` | `bundleIdentifier` | `String` | `"com.nexa.nexaapp"` | Reverse-DNS iOS app ID, derived from the project name by default. |
| `ios` | `appGroupIdentifier` | `String` | `—` | App Group identifier for plugins and widgets that share storage. |
| `ios` | `icon` | `String` or `Array<String>` | `—` | Primary and alternate app icon asset names. |
| `ios` | `arch` | `String` or `Array<String>` | `—` | Requested iOS architectures. Device builds require `arm64`. |
| `android` | `minSdk` | `Int32` | `23` | Minimum supported Android API level. Must be at least every used plugin's `minSdk`. |
| `android` | `targetSdk` | `Int32` | `36` | Target Android API level. Nexa currently requires `36`. |
| `android` | `applicationId` | `String` | `"com.nexa.nexaapp"` | Android package name, derived from the project name by default. |
| `android` | `icon` | `String` or `Array<String>` | `—` | Primary and alternate app icon asset names. |
| `android` | `arch` | `String` or `Array<String>` | `—` | Requested Android ABIs. |
| `android.cronet` | `provider` | `String` | `"play-services"` | `"play-services"` or `"embedded"` network provider. |
| `android.cronet` | `diskCacheSizeMb` | `Int32` | `64` | Cronet disk cache size in MiB. |
| `permissions` | `<permission>` | `String` | `—` | User-facing purpose message for a native permission used by the app. |
| `dependencies.<alias>` | `id`, `path` | strings | `—` | Local plugin package identity and path. |
| `dependencies.<alias>` | `id`, `git`, `rev`, `package` | strings | `—` | Pinned Git plugin source and optional package subdirectory. `rev` is required with `git`. |
| `plugins.<alias>` | `scalar option values` | string, boolean, or string array | `—` | Compile-time plugin options validated against the plugin contract. |
<!-- nexadoc:end config-options -->

---

## 5. Development Workflow & CLI

```bash
# 1. Type check the app for both targets
nexa check

# 2. Start live development with hot reload
nexa dev

# 3. Target a specific platform
nexa dev --ios
nexa dev --android
```

### CLI Command Matrix

> Generated from the CLI's own command and flag tables. Run
> `cargo test -p nexa-cli --test project_config_reference` with `NEXA_UPDATE_SNAPSHOTS=1` to
> regenerate.

<!-- nexadoc:begin cli-commands -->
| Command | Key Flags | Description |
|---|---|---|
| `nexa create <name>` | `--directory <path>` | Creates the app source, configuration, and local signing template. |
| `nexa check` | `--ios`, `--android`, `--locked`, `--deny-warnings`, `--audit` | Validates source, plugin dependencies, types, and platform constraints. |
| `nexa fmt` | `--check` | Formats Nexa source files or checks whether they need formatting. |
| `nexa dev` | `--ios`, `--android`, `--all`, `--platform <ios\|android\|all>`, `--arch <arch>`, `--once`, `--compile-only`, `--flavor <name>`, `--staging`, `--out <directory>`, `--locked` | Builds and launches a native app with DevRuntime hot reload. |
| `nexa test` | `--unit-only`, `--ios`, `--android`, `--all`, `--platform <ios\|android\|all>`, `--arch <arch>`, `--flavor <name>`, `--staging`, `--out <directory>`, `--locked` | Runs app tests; `--unit-only` skips native test hosts. |
| `nexa build` | `--release`, `--ipa`, `--aab`, `--ios`, `--android`, `--all`, `--platform <ios\|android\|all>`, `--arch <arch>`, `--flavor <name>`, `--staging`, `--out <directory>`, `--locked` | Builds a signed iOS IPA or Android AAB. |
| `nexa release` | `--ios`, `--android`, `--all`, `--platform <ios\|android\|all>`, `--arch <arch>`, `--flavor <name>`, `--staging`, `--out <directory>`, `--locked` | Builds an iOS archive or Android AAB using platform signing credentials. |
| `nexa doctor` | `—` | Verifies toolchain installation and environment health. |
| `nexa audit <file>` | `--target <ios\|android\|all>`, `--release-sizes`, `--out <path>` | Reports reachable features, generated dependencies, and optional release-size data. |
| `nexa plugin <subcommand>` | `init <plugin.id> --out <directory>`, `check <package-directory\|native.nxid>`, `generate <package-directory\|native.nxid> --target <swift\|kotlin\|cpp>` | Scaffolds, validates, and generates bindings for a plugin package. |
<!-- nexadoc:end cli-commands -->

### Interactive DevRuntime Shortcuts

While `nexa dev` is running in your terminal:

| Key Shortcut | Action | Description |
|---|---|---|
| `r` | **Hot Reload** | Updates views, functions, state initializers, and component trees instantly. |
| `Shift + R` | **Hot Restart** | Clears in-memory state and restarts the application flow. |
| `b` | **Rebuild Native Host**| Re-runs native compiler (Xcode/Gradle) when plugins or native code change. |
| `p` | **Toggle Overlay** | Toggles the on-device live FPS and frame latency diagnostics HUD. |

When a source compile fails, the running debug app keeps the last working screen and shows a tappable error banner. Open its details and choose **Open in editor** to ask the `nexa dev` host to open the reported file and line through the desktop's `vscode://` URL handler.

---

## 6. Your First Complete App

This task board demonstrates typed state, input, filtering, immutable struct updates, and list rendering:

```nexa
struct TaskItem {
    id: Int32,
    title: String,
    priority: String,
    isCompleted: Bool,
    dueLabel: String,
}

app PulseTasks {
    state tasks: Array<TaskItem> = [
        TaskItem(1, "Review the release checklist", "High", false, "Today"),
        TaskItem(2, "Send design feedback", "Normal", true, "Yesterday")
    ]
    state draftTitle: String = ""
    state showCompleted: Bool = false

    body {
        Column(spacing: 16, padding: 16) {
            Text("Today's tasks", fontSize: 26, fontWeight: Bold)
            Row(spacing: 8) {
                TextInput(value: draftTitle, placeholder: "Add a task")
                Button("Add task", icon: "add", disabled: draftTitle == "") {
                    if draftTitle != "" {
                        tasks.append(TaskItem(tasks.count + 1, draftTitle, "Normal", false, "Today"))
                        draftTitle = ""
                    }
                }
            }
            Switch(value: showCompleted, label: "Show completed tasks")

            FastList(tasks.filter { task -> showCompleted || !task.isCompleted }, key: .id) { task, index in
                Row(spacing: 12, padding: 12, background: "#F1F5F9", cornerRadius: 10) {
                    Button(task.isCompleted ? "Reopen" : "Complete", style: Plain) {
                        tasks = tasks.map { current ->
                            if current.id == task.id {
                                TaskItem(current.id, current.title, current.priority, !current.isCompleted, current.dueLabel)
                            } else {
                                current
                            }
                        }
                    }
                    Column(spacing: 4) {
                        Text("\(index + 1). \(task.title)", fontWeight: Semibold)
                        Text(task.dueLabel, fontSize: 13, color: "#64748B")
                    }
                    Spacer()
                    Row(padding: 6, background: "#E2E8F0", cornerRadius: 6) {
                        Text(task.priority, fontSize: 12, fontWeight: Semibold, color: "#334155")
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
- Browse the [**Component Guide**](components.md) for native layout and interaction patterns, and the generated [**Syntax Audit**](syntax-audit.md) for every accepted component option and modifier.
- Check [**State & Navigation**](state-and-navigation.md) for multi-screen stacks, modal sheets, and reactive signals.
