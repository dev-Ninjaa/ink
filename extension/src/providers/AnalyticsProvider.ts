import * as vscode from "vscode";
import { toInkError } from "../errors/InkError";
import { CacheStats } from "../models/CacheStats";
import { ExecutionReport } from "../models/ExecutionReport";
import { Logger } from "../services/Logger";
import { RuntimeManager } from "../services/RuntimeManager";
import { ExtensionStateService } from "../state/ExtensionStateService";
import { renderAnalyticsWebview } from "../webviews/AnalyticsWebview";

export class AnalyticsProvider implements vscode.WebviewViewProvider {
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

    this.stateService.setSelectedView("analytics");
    try {
      this.runtimeManager.getSettings();
      const [analytics, health] = await Promise.all([
        this.runtimeManager.getAnalytics(),
        this.runtimeManager.getHealth()
      ]);
      const workspace = this.runtimeManager.getWorkspaceInfo();

      this.view.webview.html = renderAnalyticsWebview(workspace, analytics.report, analytics.cache, health);
    } catch (error) {
      const inkError = toInkError(error, "Runtime");
      this.logger.error("Analytics refresh failed.", inkError);
      this.view.webview.html = renderAnalyticsWebview(
        this.runtimeManager.getWorkspaceInfo(),
        this.emptyReport(),
        this.emptyCache()
      );
    }
  }

  private emptyReport(): ExecutionReport {
    return {
      generatedAt: new Date(),
      timeline: [],
      runtimeStatistics: {
        totalRuns: 0,
        averageExecutionTimeMs: 0,
        failedRuns: 0,
        successfulRuns: 0
      }
    };
  }

  private emptyCache(): CacheStats {
    return {
      entries: [],
      cacheSizeKb: 0,
      hitRate: 0
    };
  }
}
