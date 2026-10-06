import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let output: vscode.OutputChannel | undefined;
const devTerminals = new Map<string, vscode.Terminal>();

export function activate(context: vscode.ExtensionContext): void {
  output = vscode.window.createOutputChannel("Nexa Language Server");
  context.subscriptions.push(output);

  const watcher = vscode.workspace.createFileSystemWatcher("**/*.nx");
  context.subscriptions.push(watcher);
  client = createLanguageClient(context, watcher, output);
  context.subscriptions.push(client);
  void startLanguageClient(client, output);

  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((event) => {
      if (!event.affectsConfiguration("nexa.lsp.serverPath")) return;
      void restartLanguageClient(context, watcher, output!);
    }),
    vscode.window.onDidCloseTerminal((terminal) => {
      for (const [folder, running] of devTerminals) {
        if (running === terminal) devTerminals.delete(folder);
      }
      updateDevContext();
    }),
    vscode.workspace.onDidChangeWorkspaceFolders(() => {
      // vscode-languageclient synchronizes workspace-folder changes itself;
      // this refreshes command context when roots are added or removed.
      updateDevContext();
    }),
    vscode.commands.registerCommand("nexa.dev.start", async () => {
      const folder = activeWorkspaceFolder();
      if (!folder) {
        void vscode.window.showErrorMessage("Open a Nexa project folder before starting Nexa Dev.");
        return;
      }
      const existing = devTerminals.get(folder.uri.toString());
      if (existing) {
        existing.show();
        return;
      }
      if (!fs.existsSync(path.join(folder.uri.fsPath, "nexa.config.nx"))) {
        void vscode.window.showErrorMessage(`No nexa.config.nx was found in ${folder.name}.`);
        return;
      }

      const cliPath = resolveCliPath(context);
      const terminal = vscode.window.createTerminal({
        name: `Nexa Dev: ${folder.name}`,
        cwd: folder.uri.fsPath,
      });
      devTerminals.set(folder.uri.toString(), terminal);
      terminal.show();
      terminal.sendText(`${shellQuote(cliPath)} dev`, true);
      updateDevContext();
    }),
    vscode.commands.registerCommand("nexa.dev.reload", () => sendDevShortcut("r")),
    vscode.commands.registerCommand("nexa.dev.hotRestart", () => sendDevShortcut("R")),
    vscode.commands.registerCommand("nexa.dev.rebuild", () => sendDevShortcut("b")),
    vscode.commands.registerCommand("nexa.dev.performanceOverlay", () => sendDevShortcut("p")),
    vscode.commands.registerCommand("nexa.dev.stop", () => {
      const folder = activeWorkspaceFolder();
      const terminal = folder ? devTerminals.get(folder.uri.toString()) : activeDevTerminal();
      if (terminal) {
        for (const [key, value] of devTerminals) {
          if (value === terminal) devTerminals.delete(key);
        }
        terminal.dispose();
      }
      updateDevContext();
    })
  );
}

export async function deactivate(): Promise<void> {
  for (const terminal of devTerminals.values()) terminal.dispose();
  devTerminals.clear();
  await client?.stop();
  client = undefined;
}

function createLanguageClient(
  context: vscode.ExtensionContext,
  watcher: vscode.FileSystemWatcher,
  outputChannel: vscode.OutputChannel
): LanguageClient {
  const config = vscode.workspace.getConfiguration("nexa");
  const suffix = process.platform === "win32" ? ".exe" : "";
  const binaryDir = path.join("bin", `${process.platform}-${process.arch}`);
  const bundledPath = context.asAbsolutePath(path.join(binaryDir, `nexa-lsp${suffix}`));
  const configuredPath = config.get<string>("lsp.serverPath")?.trim();
  const serverPath = configuredPath || (fs.existsSync(bundledPath) ? bundledPath : "nexa-lsp");
  const serverOptions: ServerOptions = {
    run: { command: serverPath, transport: TransportKind.stdio },
    debug: { command: serverPath, transport: TransportKind.stdio },
  };
  const clientOptions: LanguageClientOptions = {
    // Omitting scheme supports local, remote, and untitled Nexa documents.
    // `nexa.config.nx` is a normal Nexa source document and uses the same LSP.
    documentSelector: [{ language: "nexa" }],
    synchronize: { fileEvents: watcher },
    traceOutputChannel: outputChannel,
    outputChannel,
  };
  return new LanguageClient("nexaLsp", "Nexa Language Server", serverOptions, clientOptions);
}

async function startLanguageClient(
  languageClient: LanguageClient,
  outputChannel: vscode.OutputChannel
): Promise<void> {
  try {
    await languageClient.start();
    outputChannel.appendLine("Nexa language server started.");
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error);
    outputChannel.appendLine(`Failed to start Nexa language server: ${detail}`);
    outputChannel.show(true);
    void vscode.window.showErrorMessage(
      `Nexa language server failed to start: ${detail}. See the Nexa Language Server output for details.`
    );
  }
}

async function restartLanguageClient(
  context: vscode.ExtensionContext,
  watcher: vscode.FileSystemWatcher,
  outputChannel: vscode.OutputChannel
): Promise<void> {
  const previous = client;
  client = undefined;
  if (previous) await previous.stop();
  const next = createLanguageClient(context, watcher, outputChannel);
  client = next;
  await startLanguageClient(next, outputChannel);
}

function resolveCliPath(context: vscode.ExtensionContext): string {
  const suffix = process.platform === "win32" ? ".exe" : "";
  const bundledPath = context.asAbsolutePath(
    path.join("bin", `${process.platform}-${process.arch}`, `nexa${suffix}`)
  );
  const configuredPath = vscode.workspace.getConfiguration("nexa.cli").get<string>("path")?.trim();
  return configuredPath || (fs.existsSync(bundledPath) ? bundledPath : "nexa");
}

function activeWorkspaceFolder(): vscode.WorkspaceFolder | undefined {
  const document = vscode.window.activeTextEditor?.document;
  return document ? vscode.workspace.getWorkspaceFolder(document.uri) : vscode.workspace.workspaceFolders?.[0];
}

function activeDevTerminal(): vscode.Terminal | undefined {
  const folder = activeWorkspaceFolder();
  if (folder) {
    const terminal = devTerminals.get(folder.uri.toString());
    if (terminal) return terminal;
  }
  return devTerminals.values().next().value as vscode.Terminal | undefined;
}

function sendDevShortcut(shortcut: string): void {
  const terminal = activeDevTerminal();
  if (!terminal) {
    void vscode.window.showInformationMessage("Start Nexa Dev in a workspace folder before using dev shortcuts.");
    return;
  }
  terminal.show();
  terminal.sendText(shortcut, false);
}

function updateDevContext(): void {
  void vscode.commands.executeCommand("setContext", "nexa.dev.running", devTerminals.size > 0);
}

function shellQuote(value: string): string {
  if (process.platform === "win32") return `"${value.replaceAll('"', '\\"')}"`;
  return `'${value.replaceAll("'", "'\\''")}'`;
}
