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
let devTerminal: vscode.Terminal | undefined;

export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("nexa");
  const serverPath = config.get<string>("lsp.serverPath") || "nexa-lsp";

  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
    debug: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "nexa" }],
    synchronize: {
      fileEvents: vscode.workspace.createFileSystemWatcher("**/*.nx"),
    },
  };

  client = new LanguageClient(
    "nexaLsp",
    "Nexa Language Server",
    serverOptions,
    clientOptions
  );

  client.start();

  context.subscriptions.push(
    vscode.window.onDidCloseTerminal((terminal) => {
      if (terminal === devTerminal) {
        devTerminal = undefined;
        void vscode.commands.executeCommand("setContext", "nexa.dev.running", false);
      }
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.start", async () => {
      if (devTerminal) {
        devTerminal.show();
        return;
      }

      const editorFolder = vscode.window.activeTextEditor
        ? vscode.workspace.getWorkspaceFolder(vscode.window.activeTextEditor.document.uri)
        : undefined;
      const folder = editorFolder || vscode.workspace.workspaceFolders?.[0];
      if (!folder) {
        void vscode.window.showErrorMessage("Open a Nexa project folder before starting `nexa dev`.");
        return;
      }
      if (!fs.existsSync(path.join(folder.uri.fsPath, "nexa.config.nx"))) {
        void vscode.window.showErrorMessage("This folder does not contain nexa.config.nx.");
        return;
      }

      const cliPath = vscode.workspace.getConfiguration("nexa.cli").get<string>("path") || "nexa";
      const escapedCliPath = cliPath.replaceAll('"', '\\"');
      devTerminal = vscode.window.createTerminal({
        name: "Nexa Dev",
        cwd: folder.uri.fsPath,
      });
      devTerminal.show();
      devTerminal.sendText(`"${escapedCliPath}" dev`, true);
      void vscode.commands.executeCommand("setContext", "nexa.dev.running", true);
    })
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.reload", () => sendDevShortcut("r"))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.hotRestart", () => sendDevShortcut("R"))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.rebuild", () => sendDevShortcut("b"))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.performanceOverlay", () => sendDevShortcut("p"))
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("nexa.dev.stop", () => {
      devTerminal?.dispose();
      devTerminal = undefined;
      void vscode.commands.executeCommand("setContext", "nexa.dev.running", false);
    })
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

function sendDevShortcut(shortcut: string) {
  if (!devTerminal) {
    void vscode.window.showInformationMessage("Start Nexa Dev before using dev shortcuts.");
    return;
  }
  devTerminal.show();
  devTerminal.sendText(shortcut, false);
}
