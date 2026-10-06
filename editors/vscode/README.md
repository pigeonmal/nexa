# Nexa VS Code Extension

Official Visual Studio Code extension providing full language tooling for **Nexa** (`.nx`).

## Features

- **Nexa and project config support**: `.nx` files and `nexa.config.nx` use the same language server and grammar, including remote and untitled editor documents.
- **Syntax Highlighting**: TextMate scopes for declarations, imports, types, control flow, config blocks and keys, built-in components, strings, and comments.
- **Context-aware language tooling**: Live diagnostics use Nexa's parsers and compiler; completion adapts to app declarations, UI/action bodies, types, component arguments, API members, and config blocks. Hover documentation and document symbols are also provided by `nexa-lsp`.
- **Server logs**: Startup errors and LSP traces are available in **View → Output → Nexa Language Server**. Tracing is controlled by `nexaLsp.trace.server`.
- **Snippets**: `nexa-app`, `nexa-config`, `nexa-component`, `nexa-screen`, `nexa-struct`, `nexa-fn`, `nexa-plugin`, `nexa-import`, `nexa-icon`, and `nexa-button`.
- **Dev workflow**: Start and stop `nexa dev` from the Command Palette. While it runs, `r` hot reloads, `Shift+R` hot restarts, `b` rebuilds and relaunches, and `p` toggles the FPS/frame-time overlay. VS Code commands are available in the Command Palette and use `Ctrl+Alt+R`, `Ctrl+Alt+Shift+R`, `Ctrl+Alt+B`, and `Ctrl+Alt+P` (macOS: `Cmd+Option` with the same suffixes).

## Installation

The platform VSIX packages the matching `nexa` CLI and `nexa-lsp` binaries. Users install the extension from the Visual Studio Marketplace or install the matching VSIX using **Extensions → … → Install from VSIX…**. No Rust toolchain or separate Nexa binary installation is needed.

The extension uses its bundled CLI and language server automatically. `nexa.cli.path` and `nexa.lsp.serverPath` can override those binaries if needed; changing the language-server path restarts the client. The client follows workspace-folder changes and supports multiple project roots. Building and launching native apps still requires the platform toolchains: Xcode for iOS and Android SDK/Gradle for Android.

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
3. Open `nexa.config.nx` or any `.nx` file. Use `nexa-app` and `nexa-config` to start from the matching scaffolds.
4. If the server does not start, open **View → Output → Nexa Language Server** for the process error and LSP trace.
