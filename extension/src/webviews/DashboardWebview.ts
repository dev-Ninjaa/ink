import { RuntimeHealth } from "../contracts";
import { Metrics } from "../models/Metrics";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { escapeHtml } from "../utils/webview";
import { renderEmptyWorkspace, renderWebviewPage } from "./WebviewPage";

export function renderDashboardWebview(workspace: WorkspaceInfo, metrics: Metrics, health?: RuntimeHealth): string {
  if (!workspace.isOpen) {
    return renderWebviewPage({ title: "Dashboard", body: renderEmptyWorkspace(), health });
  }

  const metricsHtml = metrics.items
    .map((item) => `
      <article class="metric">
        <div>
          <div class="metric-label">${escapeHtml(item.label)}</div>
          <div class="metric-value">${escapeHtml(item.value)}</div>
        </div>
        <p class="metric-detail">${escapeHtml(item.detail)}</p>
      </article>
    `)
    .join("");

  return renderWebviewPage({ title: "Dashboard", health, body: `
    <section class="header">
      <div>
        <h1>Ink Dashboard</h1>
        <p class="muted">Live orchestration metrics for the current workspace.</p>
      </div>
      <div class="workspace">
        <h2>Project</h2>
        <p>${escapeHtml(workspace.name)}</p>
      </div>
    </section>
    <section class="grid">${metricsHtml}</section>
  ` });
}
