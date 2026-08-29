import * as vscode from "vscode";
import { toInkError } from "../errors/InkError";
import { Logger } from "../services/Logger";
import { RuntimeManager } from "../services/RuntimeManager";
import { ExtensionStateService } from "../state/ExtensionStateService";
import { renderDashboardWebview } from "../webviews/DashboardWebview";

export class DashboardProvider implements vscode.WebviewViewProvider {
  private view?: vscode.WebviewView;

  constructor(
    private readonly runtimeManager: RuntimeManager,
    private readonly stateService: ExtensionStateService,
    private readonly logger: Logger
  ) {}

  async resolveWebviewView(webviewView: vscode.WebviewView): Promise<void> {
    this.view = webviewView;
    webviewView.webview.options = {
      enableScripts: false
    };

    await this.refresh();
  }

  async refresh(): Promise<void> {
    if (!this.view) {
      return;
    }

    this.stateService.setSelectedView("dashboard");
    try {
      this.runtimeManager.getSettings();
      const [metricsResponse, health] = await Promise.all([
        this.runtimeManager.getMetrics(),
        this.runtimeManager.getHealth()
      ]);
      const workspace = this.runtimeManager.getWorkspaceInfo();

      this.view.webview.html = renderDashboardWebview(workspace, metricsResponse.metrics, health);
    } catch (error) {
      const inkError = toInkError(error, "Runtime");
      this.logger.error("Dashboard refresh failed.", inkError);
      this.view.webview.html = renderDashboardWebview(this.runtimeManager.getWorkspaceInfo(), { tokensSaved: 0, cacheHitRate: 0, parallelTasks: 0, executionTimeMs: 0, contextReductionPercent: 0, items: [] });
    }
  }
}
