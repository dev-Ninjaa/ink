import * as vscode from "vscode";
import { getUserMessage, toInkError } from "../errors/InkError";
import { Logger } from "../services/Logger";
import { RuntimeManager } from "../services/RuntimeManager";
import { DependencyGraphPanel } from "../providers/DependencyGraphPanel";
import { OptimizePanel } from "../providers/OptimizePanel";

interface ProviderRefreshers {
  refreshDashboard(): Promise<void>;
  refreshAnalytics(): Promise<void>;
  refreshAgents(): void;
  refreshCache(): void;
}

export function registerCommands(
  context: vscode.ExtensionContext,
  runtimeManager: RuntimeManager,
  logger: Logger,
  refreshers: ProviderRefreshers,
  graphPanel: DependencyGraphPanel,
  optimizePanel: OptimizePanel
): void {
  const register = (command: string, callback: () => Promise<void>) => {
    context.subscriptions.push(vscode.commands.registerCommand(command, async () => {
      logger.info(`Command executed: ${command}`);
      try {
        await callback();
      } catch (error) {
        const inkError = toInkError(error, "Unknown");
        logger.error(`Command failed: ${command}`, inkError);
        void vscode.window.showErrorMessage(getUserMessage(inkError));
      }
    }));
  };

  register("ink.openDashboard", async () => {
    await vscode.commands.executeCommand("workbench.view.extension.ink");
    await vscode.commands.executeCommand("ink.dashboard.focus");
    await refreshers.refreshDashboard();
  });

  registerWorkspaceCommand("ink.optimizeContext", async () => {
    const response = await runtimeManager.optimizeContext();
    if (response.result) {
      const result = response.result;
      await optimizePanel.show(runtimeManager.getWorkspaceInfo(), result);
      return `Context optimized: ${result.metrics.filesSelected}/${result.metrics.filesConsidered} files, ${result.metrics.tokenReductionPercent.toFixed(1)}% token reduction.`;
    }
    return "Context optimization complete.";
  });

  registerWorkspaceCommand("ink.buildDependencyGraph", async () => {
    const { graph } = await runtimeManager.buildDependencyGraph();
    await graphPanel.show(runtimeManager.getWorkspaceInfo(), graph);
    return `Dependency graph built (${graph.nodes.length} nodes, ${graph.edges.length} edges).`;
  });

  registerWorkspaceCommand("ink.scheduleAgents", async () => {
    const { agents } = await runtimeManager.scheduleAgents();
    refreshers.refreshAgents();
    const scheduled = agents.active.length + agents.pending.length + agents.completed.length;
    return `Scheduled ${scheduled} agent${scheduled === 1 ? "" : "s"}.`;
  });

  registerWorkspaceCommand("ink.generateReport", async () => {
    await runtimeManager.generateReport();
    await refreshers.refreshAnalytics();
    return "Execution report generated.";
  });

  registerWorkspaceCommand("ink.clearCache", async () => {
    await runtimeManager.clearCache();
    refreshers.refreshCache();
    return "Cache cleared.";
  });

  function registerWorkspaceCommand(command: string, action: () => Promise<string>): void {
    register(command, async () => {
      const workspace = runtimeManager.getWorkspaceInfo();
      if (!workspace.isOpen) {
        void vscode.window.showInformationMessage("Open a project to use Ink.");
        return;
      }

      const message = await action();
      void vscode.window.showInformationMessage(message);
    });
  }
}
