import { RuntimeHealth } from "../contracts";
import { escapeHtml, getNonce } from "../utils/webview";
import { baseStyles } from "./styles";

export interface WebviewPageOptions {
  readonly title: string;
  readonly body: string;
  readonly health?: RuntimeHealth;
}

export function renderWebviewPage(options: WebviewPageOptions): string {
  const nonce = getNonce();
  const healthHtml = options.health ? renderHealthIndicator(options.health) : "";

  return `<!DOCTYPE html>
  <html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'nonce-${nonce}';">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>${escapeHtml(options.title)}</title>
    <style nonce="${nonce}">${baseStyles}</style>
  </head>
  <body>
    <main class="shell">
      ${healthHtml}
      ${options.body}
    </main>
  </body>
  </html>`;
}

export function renderEmptyWorkspace(): string {
  return `<section class="empty">Open a project to use Ink.</section>`;
}

function renderHealthIndicator(health: RuntimeHealth): string {
  const items = health.components
    .map((component) => `
      <span class="health-item">
        <strong>${escapeHtml(component.name)}:</strong> ${escapeHtml(component.message)}
      </span>
    `)
    .join("");

  return `<section class="health" aria-label="Runtime health">${items}</section>`;
}
