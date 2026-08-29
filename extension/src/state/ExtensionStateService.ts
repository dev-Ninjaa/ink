import * as vscode from "vscode";
import { RuntimeHealth, RuntimeStatus } from "../contracts";
import { WorkspaceInfo } from "../models/WorkspaceInfo";

export type InkViewId = "dashboard" | "analytics" | "agents" | "cache";

export interface ExtensionStateSnapshot {
  readonly runtimeStatus: RuntimeStatus;
  readonly workspace: WorkspaceInfo;
  readonly selectedView: InkViewId;
  readonly health?: RuntimeHealth;
}

export class ExtensionStateService implements vscode.Disposable {
  private readonly didChangeEmitter = new vscode.EventEmitter<ExtensionStateSnapshot>();
  private snapshot: ExtensionStateSnapshot;

  readonly onDidChange = this.didChangeEmitter.event;

  constructor(initialState: ExtensionStateSnapshot) {
    this.snapshot = initialState;
  }

  getSnapshot(): ExtensionStateSnapshot {
    return this.snapshot;
  }

  setRuntimeStatus(runtimeStatus: RuntimeStatus): void {
    this.update({ runtimeStatus });
  }

  setWorkspace(workspace: WorkspaceInfo): void {
    this.update({ workspace });
  }

  setSelectedView(selectedView: InkViewId): void {
    this.update({ selectedView });
  }

  setHealth(health: RuntimeHealth): void {
    this.update({ health });
  }

  dispose(): void {
    this.didChangeEmitter.dispose();
  }

  private update(next: Partial<ExtensionStateSnapshot>): void {
    this.snapshot = { ...this.snapshot, ...next };
    this.didChangeEmitter.fire(this.snapshot);
  }
}
