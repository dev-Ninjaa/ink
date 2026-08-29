import * as vscode from "vscode";
import { InkError } from "../errors/InkError";
import { EventBus } from "../events/EventBus";

export interface InkSettings {
  readonly enableCache: boolean;
  readonly enableAnalytics: boolean;
  readonly enableParallelism: boolean;
  readonly maxAgents: number;
}

export interface McpServerSettings {
  readonly transport: "stdio" | "http";
  readonly command: string;
  readonly args: readonly string[];
  readonly url: string;
}

export class SettingsService implements vscode.Disposable {
  private readonly didChangeEmitter = new vscode.EventEmitter<InkSettings>();
  private readonly disposables: vscode.Disposable[] = [];

  readonly onDidChange = this.didChangeEmitter.event;

  constructor(private readonly eventBus: EventBus) {
    this.disposables.push(vscode.workspace.onDidChangeConfiguration((event) => {
      if (event.affectsConfiguration("ink")) {
        const settings = this.getSettings();
        this.didChangeEmitter.fire(settings);
        this.eventBus.publish("SettingsChanged", settings);
      }
    }));
  }

  getSettings(): InkSettings {
    const configuration = vscode.workspace.getConfiguration("ink");
    const settings: InkSettings = {
      enableCache: configuration.get<boolean>("enableCache", true),
      enableAnalytics: configuration.get<boolean>("enableAnalytics", true),
      enableParallelism: configuration.get<boolean>("enableParallelism", true),
      maxAgents: configuration.get<number>("maxAgents", 4)
    };

    this.validate(settings);
    return settings;
  }

  getMcpServerSettings(): McpServerSettings {
    const configuration = vscode.workspace.getConfiguration("ink.mcpServer");
    const transport = configuration.get<string>("transport", "stdio");
    return {
      transport: transport === "http" ? "http" : "stdio",
      command: configuration.get<string>("command", ""),
      args: configuration.get<string[]>("args", ["--transport", "stdio"]),
      url: configuration.get<string>("url", "")
    };
  }

  validate(settings: InkSettings): void {
    if (!Number.isInteger(settings.maxAgents) || settings.maxAgents < 1 || settings.maxAgents > 32) {
      throw new InkError({
        code: "INK_INVALID_MAX_AGENTS",
        message: "ink.maxAgents must be an integer between 1 and 32.",
        category: "Configuration",
        details: { maxAgents: settings.maxAgents }
      });
    }
  }

  dispose(): void {
    this.didChangeEmitter.dispose();
    for (const disposable of this.disposables) {
      disposable.dispose();
    }
  }
}
