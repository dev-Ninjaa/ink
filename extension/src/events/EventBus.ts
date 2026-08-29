import * as vscode from "vscode";
import { RuntimeHealth, RuntimeStatus } from "../contracts";
import type { InkSettings } from "../services/SettingsService";
import { WorkspaceInfo } from "../models/WorkspaceInfo";

export interface InkEventMap {
  RuntimeStarted: RuntimeStatus;
  RuntimeStopped: RuntimeStatus;
  HealthChanged: RuntimeHealth;
  WorkspaceChanged: WorkspaceInfo;
  SettingsChanged: InkSettings;
}

export type InkEventName = keyof InkEventMap;

export interface InkEvent<TName extends InkEventName = InkEventName> {
  readonly name: TName;
  readonly payload: InkEventMap[TName];
  readonly timestamp: Date;
}

export class EventBus implements vscode.Disposable {
  private readonly emitter = new vscode.EventEmitter<InkEvent>();
  readonly event = this.emitter.event;

  publish<TName extends InkEventName>(name: TName, payload: InkEventMap[TName]): void {
    this.emitter.fire({ name, payload, timestamp: new Date() });
  }

  dispose(): void {
    this.emitter.dispose();
  }
}
