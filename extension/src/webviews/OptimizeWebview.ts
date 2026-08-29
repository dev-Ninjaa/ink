import { RuntimeHealth } from "../contracts";
import { OptimizeResult } from "../models/OptimizeResult";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { escapeHtml, getNonce } from "../utils/webview";
import { baseStyles } from "./styles";

export function renderOptimizePage(workspace: WorkspaceInfo, result: OptimizeResult, health?: RuntimeHealth): string {
  const nonce = getNonce();

  const fileRow = (file: OptimizeResult["selectedFiles"][number], showReasons: boolean) => `
    <div class="opt-file">
      <div class="opt-file-head">
        <strong>${escapeHtml(file.path)}</strong>
        <span class="muted">${file.tokens.toLocaleString("en")} tok · ${(file.relevance * 100).toFixed(0)}%</span>
      </div>
      ${showReasons && file.reasons.length > 0
        ? `<div class="chips">${file.reasons.map((reason) => `<span class="chip">${escapeHtml(reason)}</span>`).join("")}</div>`
        : ""}
    </div>`;

  const selectedHtml = result.selectedFiles.map((file) => fileRow(file, true)).join("");
  const droppedHtml = result.droppedFiles.slice(0, 40).map((file) => fileRow(file, false)).join("");
  const droppedNote = result.droppedFiles.length > 40
    ? `<p class="muted">…and ${result.droppedFiles.length - 40} more.</p>`
    : "";
  const warningsHtml = result.warnings.length === 0
    ? ""
    : `<section class="panel stack"><h2>Warnings</h2>${result.warnings
        .map((warning) => `<p>${escapeHtml(warning)}</p>`)
        .join("")}</section>`;
  const budgetClass = result.withinBudget ? "ok" : "over";

  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}';">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Ink Context Optimization — ${escapeHtml(workspace.name)}</title>
  <style nonce="${nonce}">${baseStyles}</style>
  <style nonce="${nonce}">
    .budget {
      display: inline-block;
      border-radius: 999px;
      padding: 2px 10px;
      font-size: 11px;
      color: var(--vscode-badge-foreground);
      background: var(--vscode-badge-background);
    }
    .columns {
      display: grid;
      grid-template-columns: 1fr 1fr;
      gap: 14px;
      align-items: start;
    }
    @media (max-width: 900px) { .columns { grid-template-columns: 1fr; } }
    .opt-file {
      border: 1px solid var(--vscode-panel-border);
      border-radius: 6px;
      padding: 10px;
      margin-bottom: 8px;
      background: var(--vscode-editorWidget-background);
    }
    .opt-file-head {
      display: flex;
      justify-content: space-between;
      gap: 10px;
      word-break: break-all;
    }
    .chips { margin-top: 6px; display: flex; flex-wrap: wrap; gap: 6px; }
    .chip {
      font-size: 10px;
      border-radius: 999px;
      padding: 1px 8px;
      color: var(--vscode-descriptionForeground);
      border: 1px solid var(--vscode-panel-border);
    }
  </style>
</head>
<body>
  <main class="shell">
    <section class="header">
      <div>
        <h1>Context Optimization</h1>
        <p class="muted">Token-budgeted selection from the context optimizer engine.</p>
        <p><span class="budget ${budgetClass}">${result.withinBudget ? "within budget" : "over budget"}</span></p>
      </div>
      <div class="workspace">
        <h2>Project</h2>
        <p>${escapeHtml(workspace.name)}</p>
      </div>
    </section>

    <section class="grid">
      <article class="metric">
        <div class="metric-label">Tokens Before → After</div>
        <div class="metric-value">${result.tokensBefore.toLocaleString("en")} → ${result.tokensAfter.toLocaleString("en")}</div>
        <p class="metric-detail">Budget ${result.budget.toLocaleString("en")} tokens</p>
      </article>
      <article class="metric">
        <div class="metric-label">Context Reduction</div>
        <div class="metric-value">${result.metrics.tokenReductionPercent.toFixed(1)}%</div>
        <p class="metric-detail">Noise pruned before it reaches the model</p>
      </article>
      <article class="metric">
        <div class="metric-label">Files Selected</div>
        <div class="metric-value">${result.metrics.filesSelected}/${result.metrics.filesConsidered}</div>
        <p class="metric-detail">Ranked by relevance to the task query</p>
      </article>
    </section>

    ${warningsHtml}

    <section class="stack">
      <h2>Query</h2>
      <div class="panel"><p>${escapeHtml(result.query)}</p></div>
    </section>

    <div class="columns">
      <section class="panel stack">
        <h2>Selected (${result.selectedFiles.length})</h2>
        ${selectedHtml || '<p class="muted">Nothing selected.</p>'}
      </section>
      <section class="panel stack">
        <h2>Dropped (${result.droppedFiles.length})</h2>
        ${droppedHtml || '<p class="muted">Nothing dropped.</p>'}
        ${droppedNote}
      </section>
    </div>
  </main>
</body>
</html>`;
}
