# Getting Started with Nexa

This guide walks you through installing Nexa, configuring your environment, and compiling your first cross-platform application.

---

## 1. Installation & Setup

### Prerequisites
- **Rust Toolchain**: Rust 1.85+ (`cargo`, `rustc`)
- **For iOS Target**: macOS with Xcode 16 or later. Generated projects default to iOS 16.0 and allow changing `minVersion` in `nexa.config.nx`.
- **For Android Target**: Android SDK Platform 37 and JDK 17 or later. Generated projects default to `minSdk` 24 and `targetSdk` 36; `minSdk` can be changed in `nexa.config.nx`.

### Building the Nexa CLI
Clone the repository and build the `nexa` command-line binary:

```bash
git clone https://github.com/nexa-lang/nexa.git
cd nexa
cargo build --release -p nexa-cli

# Symlink or copy the binary to your PATH:
sudo cp target/release/nexa /usr/local/bin/
```

Verify installation:
```bash
nexa --version
```

---

## 2. Creating Your First App

Create a file named `App.nx`:

```nexa
app HelloWorld {
    state count: Int32 = 0

    body {
        Column(spacing: 16) {
            Text("Welcome to Nexa!")
            Text("Current count: ${count}")

            Row(spacing: 12) {
                Button("Increment") {
                    count = count + 1
                }
                Button("Reset") {
                    count = 0
                }
            }

            if count > 5 {
                Text("You reached more than 5 taps!")
            }
        }
    }
}
```

---

## 3. Validating & Compiling

### Type Checking & Semantic Verification
To validate syntax, type inference, and semantic integrity without building:

```bash
nexa check App.nx
```

### Compiling to Native Code

To generate native SwiftUI source code:
```bash
nexa build App.nx --target swift --out AppView.swift
```

To generate native Jetpack Compose source code:
```bash
nexa build App.nx --target kotlin --out AppScreen.kt
```

### Generating native projects

Generate the native iOS and Android projects from your `.nx` app:

```bash
nexa generate App.nx --target all --out build
```

Open `build/ios/<AppName>.xcodeproj` in Xcode to build and run on an iOS Simulator or device. Open the generated `build/android` project in Android Studio, or build it with Gradle, to run on an emulator or device. `nexa generate` creates the native project files; it does not launch the app itself.

---

## 4. IDE Tooling & VS Code Extension

For real-time syntax highlighting, type diagnostics, autocomplete, and symbol outlines:

1. Build the Language Server:
   ```bash
   cargo build --release -p nexa-lsp
   sudo cp target/release/nexa-lsp /usr/local/bin/
   ```
2. Open the `editors/vscode` directory in VS Code and install the extension bundle. All `.nx` files will immediately receive rich IDE capabilities!
