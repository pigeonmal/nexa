# Nexa VS Code Extension

Official Visual Studio Code extension providing full language tooling for **Nexa** (`.nx`).

## Features

- **Syntax Highlighting**: Full TextMate grammar for `.nx` declarative syntax, control flow, built-in layout containers, and primitives.
- **Real-Time Diagnostics**: As-you-type syntax error parsing (`nexa-syntax`) and semantic validation checks (`nexa-compiler`).
- **Completion Suggestions**: Static completion templates for common UI constructs, keywords, and built-in types.
- **Hover Documentation**: Descriptions for selected layout containers, controls, state declarations, and built-in types.
- **Document Symbols & Outline**: Full hierarchical symbol outline for VS Code's Outline panel and Breadcrumbs view (`app`, `component`, `struct`, `enum`, `state`, `fn`).
- **Dev workflow**: Start and stop `nexa dev` from the Command Palette. While it runs, `r` hot reloads, `Shift+R` hot restarts, `b` rebuilds and relaunches, and `p` toggles the FPS/frame-time overlay. VS Code commands are available in the Command Palette and use `Ctrl+Alt+R`, `Ctrl+Alt+Shift+R`, `Ctrl+Alt+B`, and `Ctrl+Alt+P` (macOS: `Cmd+Option` with the same suffixes).

## Prerequisites

Ensure the `nexa-lsp` binary is built and available on your `PATH`:

```bash
cargo build --release -p nexa-lsp
# Copy or symlink target/release/nexa-lsp into /usr/local/bin or ~/.cargo/bin
```

Alternatively, configure the explicit path in your VS Code settings:

```json
{
  "nexa.lsp.serverPath": "/path/to/nexa/target/release/nexa-lsp"
}
```

The dev commands use `nexa` from `PATH` by default. Set `nexa.cli.path` if the CLI is installed elsewhere. Run **Nexa: Start Dev**, **Nexa: Hot Reload**, **Nexa: Hot Restart**, **Nexa: Rebuild and Relaunch App**, **Nexa: Toggle Performance Overlay**, or **Nexa: Stop Dev** from the Command Palette.

## Development & Building Extension

```bash
cd editors/vscode
npm install
npm run compile
```

To run and debug the extension:
1. Open `editors/vscode` in VS Code.
2. Press `F5` to launch an Extension Development Host window.
3. Open any `.nx` file to verify completions, hover, symbols, and diagnostics.
