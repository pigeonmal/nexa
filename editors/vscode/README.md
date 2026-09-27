# Nexa VS Code Extension

Official Visual Studio Code extension providing full language tooling for **Nexa** (`.nx`).

## Features

- **Syntax Highlighting**: Full TextMate grammar for `.nx` declarative syntax, control flow, built-in layout containers, and primitives.
- **Real-Time Diagnostics**: As-you-type syntax error parsing (`nexa-syntax`) and semantic validation checks (`nexa-compiler`).
- **Completion Suggestions**: Static completion templates for common UI constructs, keywords, and built-in types.
- **Hover Documentation**: Descriptions for selected layout containers, controls, state declarations, and built-in types.
- **Document Symbols & Outline**: Full hierarchical symbol outline for VS Code's Outline panel and Breadcrumbs view (`app`, `component`, `struct`, `enum`, `state`, `fn`).
- **Dev workflow**: Start and stop `nexa dev` from the Command Palette. While it runs, `r` hot reloads, `Shift+R` hot restarts, `b` rebuilds and relaunches, and `p` toggles the FPS/frame-time overlay. VS Code commands are available in the Command Palette and use `Ctrl+Alt+R`, `Ctrl+Alt+Shift+R`, `Ctrl+Alt+B`, and `Ctrl+Alt+P` (macOS: `Cmd+Option` with the same suffixes).

## Installation

The platform VSIX packages the matching `nexa` CLI and `nexa-lsp` binaries. Users install the extension from the Visual Studio Marketplace or install the matching VSIX using **Extensions → … → Install from VSIX…**. No Rust toolchain or separate Nexa binary installation is needed.

The extension uses its bundled CLI and language server automatically. `nexa.cli.path` and `nexa.lsp.serverPath` can override those binaries if needed. Building and launching native apps still requires the platform toolchains: Xcode for iOS and Android SDK/Gradle for Android.

## Development & Building Extension

```bash
cd editors/vscode
npm install
npm run compile
npm run package:extension
```

Packaging builds the CLI and language server for the current OS and CPU architecture, then creates a matching `.vsix` in this directory. Run the command on each platform you intend to distribute; the resulting VSIX is platform-specific.

To run and debug the extension:
1. Open `editors/vscode` in VS Code.
2. Press `F5` to launch an Extension Development Host window.
3. Open any `.nx` file to verify completions, hover, symbols, and diagnostics.
