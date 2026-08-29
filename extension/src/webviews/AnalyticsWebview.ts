import { RuntimeHealth } from "../contracts";
import { CacheStats } from "../models/CacheStats";
import { ExecutionReport } from "../models/ExecutionReport";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { formatKilobytes, formatMilliseconds } from "../utils/format";
import { escapeHtml } from "../utils/webview";
import { renderEmptyWorkspace, renderWebviewPage } from "./WebviewPage";

export function renderAnalyticsWebview(workspace: WorkspaceInfo, report: ExecutionReport, cache: CacheStats, health?: RuntimeHealth): string {
  if (!workspace.isOpen) {
    return renderWebviewPage({ title: "Analytics", body: renderEmptyWorkspace(), health });
  }

  const timelineHtml = report.timeline
    .map((event) => `
      <div class="timeline-item">
        <div>
          <strong>${escapeHtml(event.label)}</strong>
          <p class="muted">${formatMilliseconds(event.durationMs)}</p>
        </div>
        <span class="status">${escapeHtml(event.status)}</span>
      </div>
    `)
    .join("");

  return renderWebviewPage({ title: "Analytics", health, body: `
    <section class="header">
      <div>
        <h1>Analytics</h1>
        <p class="muted">Execution, cache, and runtime statistics from the runtime service.</p>
      </div>
      <div class="workspace">
        <h2>Project</h2>
        <p>${escapeHtml(workspace.name)}</p>
      </div>
    </section>
    <section class="panel stack">
      <h2>Execution Timeline</h2>
      ${timelineHtml}
    </section>
    <section class="grid">
      <article class="metric">
        <div class="metric-label">Cache Statistics</div>
        <div class="metric-value">${cache.hitRate}%</div>
        <p class="metric-detail">${cache.entries.length} entries, ${formatKilobytes(cache.cacheSizeKb)} stored</p>
      </article>
      <article class="metric">
        <div class="metric-label">Runtime Statistics</div>
        <div class="metric-value">${report.runtimeStatistics.successfulRuns}/${report.runtimeStatistics.totalRuns}</div>
        <p class="metric-detail">${formatMilliseconds(report.runtimeStatistics.averageExecutionTimeMs)} average execution time</p>
      </article>
    </section>
  ` });
}
