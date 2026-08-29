import * as vscode from "vscode";
import { DependencyGraph } from "../models/DependencyGraph";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { Logger } from "../services/Logger";
import { renderDependencyGraphPage } from "../webviews/DependencyGraphWebview";

export class DependencyGraphPanel {
  private panel?: vscode.WebviewPanel;

  constructor(private readonly logger: Logger) {}

  async show(workspace: WorkspaceInfo, graph: DependencyGraph): Promise<void> {
    if (!this.panel) {
      this.panel = vscode.window.createWebviewPanel(
        "ink.dependencyGraph",
        "Ink Dependency Graph",
        vscode.ViewColumn.Active,
        { enableScripts: true }
      );
      this.panel.onDidDispose(() => {
        this.panel = undefined;
      });
    }

    this.panel.title = `Ink Graph — ${workspace.name}`;
    this.panel.webview.html = renderDependencyGraphPage(workspace, graph);
    this.logger.info(`Rendered dependency graph for ${workspace.name}: ${graph.nodes.length} nodes, ${graph.edges.length} edges.`);
  }

  dispose(): void {
    this.panel?.dispose();
    this.panel = undefined;
  }
}
