# Nexa VS Code Extension

Official Visual Studio Code extension providing full language tooling for **Nexa** (`.nx`).

## Features

- **Syntax Highlighting**: Full TextMate grammar for `.nx` declarative syntax, control flow, built-in layout containers, and primitives.
- **Real-Time Diagnostics**: As-you-type syntax error parsing (`nexa-syntax`) and semantic validation checks (`nexa-compiler`).
- **IntelliSense & Autocompletion**: Context-aware completion items for declarative containers (`Column`, `Row`, `Stack`, `LazyList`, etc.), interactive controls (`Button`, `TextField`, `Toggle`, etc.), keywords, and primitive types.
- **Hover Documentation**: Detailed Markdown documentation and code examples for layout containers, error propagation (`Result<T, E>`), and keywords.
- **Document Symbols & Outline**: Full hierarchical symbol outline for VS Code's Outline panel and Breadcrumbs view (`app`, `component`, `struct`, `enum`, `state`, `fn`).

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
