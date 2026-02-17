import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: vscode.ExtensionContext) {
  const config = vscode.workspace.getConfiguration("rholang.lsp");
  const serverPath = config.get<string>("path", "rholang-lsp");

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: ["--stdio"],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "rholang" }],
  };

  client = new LanguageClient(
    "rholang-lsp",
    "Rholang Language Server",
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  return client?.stop();
}
