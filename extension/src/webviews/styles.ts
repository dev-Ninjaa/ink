export const baseStyles = `
  :root {
    color-scheme: light dark;
  }

  body {
    margin: 0;
    padding: 20px;
    color: var(--vscode-foreground);
    background: var(--vscode-editor-background);
    font-family: var(--vscode-font-family);
  }

  .shell {
    display: flex;
    flex-direction: column;
    gap: 18px;
  }

  .header {
    display: flex;
    justify-content: space-between;
    gap: 16px;
    align-items: flex-start;
    border-bottom: 1px solid var(--vscode-panel-border);
    padding-bottom: 14px;
  }

  h1, h2, h3, p {
    margin: 0;
  }

  h1 {
    font-size: 22px;
    font-weight: 650;
  }

  h2 {
    font-size: 14px;
    font-weight: 650;
    color: var(--vscode-descriptionForeground);
    text-transform: uppercase;
  }

  .muted {
    color: var(--vscode-descriptionForeground);
  }

  .workspace {
    text-align: right;
    min-width: 140px;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px;
  }

  .metric, .panel, .timeline-item {
    border: 1px solid var(--vscode-panel-border);
    border-radius: 6px;
    background: var(--vscode-editorWidget-background);
  }

  .metric {
    padding: 14px;
    min-height: 96px;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    gap: 10px;
  }

  .metric-value {
    font-size: 24px;
    font-weight: 700;
  }

  .metric-label {
    font-size: 12px;
    color: var(--vscode-descriptionForeground);
  }

  .metric-detail {
    font-size: 12px;
    color: var(--vscode-descriptionForeground);
    line-height: 1.4;
  }

  .panel {
    padding: 14px;
  }

  .stack {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .timeline-item {
    padding: 12px;
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 8px;
    align-items: center;
  }

  .status {
    font-size: 11px;
    color: var(--vscode-badge-foreground);
    background: var(--vscode-badge-background);
    border-radius: 999px;
    padding: 2px 8px;
  }

  .empty {
    border: 1px solid var(--vscode-panel-border);
    border-radius: 6px;
    padding: 18px;
    color: var(--vscode-descriptionForeground);
  }

  .health {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    align-items: center;
    border: 1px solid var(--vscode-panel-border);
    border-radius: 6px;
    padding: 8px 10px;
    background: var(--vscode-sideBar-background);
    color: var(--vscode-descriptionForeground);
    font-size: 12px;
  }

  .health-item {
    white-space: nowrap;
  }
`;
