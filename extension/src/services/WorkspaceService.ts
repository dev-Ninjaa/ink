import * as vscode from "vscode";
import { EventBus } from "../events/EventBus";
import { WorkspaceInfo } from "../models/WorkspaceInfo";

export class WorkspaceService implements vscode.Disposable {
  private readonly didChangeEmitter = new vscode.EventEmitter<WorkspaceInfo>();
  private readonly workspaceFoldersDisposable: vscode.Disposable;

  readonly onDidChange = this.didChangeEmitter.event;

  constructor(private readonly eventBus?: EventBus) {
    this.workspaceFoldersDisposable = vscode.workspace.onDidChangeWorkspaceFolders(() => {
      const workspace = this.getWorkspaceInfo();
      this.didChangeEmitter.fire(workspace);
      this.eventBus?.publish("WorkspaceChanged", workspace);
    });
  }

  getWorkspaceInfo(): WorkspaceInfo {
    const folders = vscode.workspace.workspaceFolders ?? [];
    const folder = folders[0];

    if (!folder) {
      return {
        isOpen: false,
        name: "No workspace",
        isMultiRoot: false,
        folders: []
      };
    }

    const workspaceFolders = folders.map((workspaceFolder) => ({
      name: workspaceFolder.name,
      path: workspaceFolder.uri.fsPath
    }));

    return {
      isOpen: true,
      name: vscode.workspace.name ?? folder.name,
      isMultiRoot: folders.length > 1,
      folders: workspaceFolders,
      path: folder.uri.fsPath
    };
  }

  dispose(): void {
    this.didChangeEmitter.dispose();
    this.workspaceFoldersDisposable.dispose();
  }
}
