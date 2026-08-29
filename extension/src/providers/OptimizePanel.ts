import * as vscode from "vscode";
import { RuntimeHealth } from "../contracts";
import { OptimizeResult } from "../models/OptimizeResult";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { Logger } from "../services/Logger";
import { renderOptimizePage } from "../webviews/OptimizeWebview";

export class OptimizePanel {
  private panel?: vscode.WebviewPanel;

  constructor(private readonly logger: Logger) {}

  async show(workspace: WorkspaceInfo, result: OptimizeResult, health?: RuntimeHealth): Promise<void> {
    if (!this.panel) {
      this.panel = vscode.window.createWebviewPanel(
        "ink.optimize",
        "Ink Context Optimization",
        vscode.ViewColumn.Active,
        { enableScripts: false }
      );
      this.panel.onDidDispose(() => {
        this.panel = undefined;
      });
    }

    this.panel.title = `Ink Optimize — ${workspace.name}`;
    this.panel.webview.html = renderOptimizePage(workspace, result, health);
    this.logger.info(
      `Rendered context optimization for ${workspace.name}: ${result.metrics.filesSelected}/${result.metrics.filesConsidered} files, ${result.metrics.tokenReductionPercent.toFixed(1)}% token reduction.`
    );
  }

  dispose(): void {
    this.panel?.dispose();
    this.panel = undefined;
  }
}
