import * as fs from "fs";
import * as path from "path";
import * as vscode from "vscode";
import { registerCommands } from "./commands/registerCommands";
import { EventBus } from "./events/EventBus";
import { AnalyticsProvider } from "./providers/AnalyticsProvider";
import { AgentsProvider } from "./providers/AgentsProvider";
import { CacheProvider } from "./providers/CacheProvider";
import { DashboardProvider } from "./providers/DashboardProvider";
import { DependencyGraphPanel } from "./providers/DependencyGraphPanel";
import { OptimizePanel } from "./providers/OptimizePanel";
import { OutputChannelLogger } from "./services/Logger";
import { McpClient, SdkMcpClient } from "./services/McpClient";
import { BinaryBootstrapService } from "./services/BinaryBootstrapService";
import { McpRuntimeFactory } from "./services/McpRuntimeFactory";
import { RuntimeHealthService } from "./services/RuntimeHealthService";
import { RuntimeManager } from "./services/RuntimeManager";
import { SettingsService } from "./services/SettingsService";
import { WorkspaceService } from "./services/WorkspaceService";
import { ExtensionStateService } from "./state/ExtensionStateService";
import { toInkError } from "./errors/InkError";
import { MockRuntimeFactory } from "./mocks/MockRuntimeFactory";

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  const logger = new OutputChannelLogger("INK");
  const eventBus = new EventBus();
  const workspaceService = new WorkspaceService(eventBus);
  const settingsService = new SettingsService(eventBus);
  const initialWorkspace = workspaceService.getWorkspaceInfo();
  const stateService = new ExtensionStateService({
    runtimeStatus: {
      state: "stopped",
      message: "Runtime stopped.",
      updatedAt: new Date()
    },
    workspace: initialWorkspace,
    selectedView: "dashboard"
  });
  const healthService = new RuntimeHealthService(workspaceService, eventBus);

  // ── Status bar item ──────────────────────────────────────────────────────────
  const statusBarItem = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 10);
  statusBarItem.text = "$(loading~spin) Ink: connecting…";
  statusBarItem.tooltip = "Ink MCP Server — click to restart";
  statusBarItem.command = "ink.restartServer";
  statusBarItem.show();
  context.subscriptions.push(statusBarItem);

  // ── Bootstrap: download binary to global storage if not found ────────────────
  const bootstrapService = new BinaryBootstrapService(context, logger);
  const bootstrapResult = await bootstrapService.ensureBinary();

  if (bootstrapResult.status === "download_failed") {
    logger.info(`Binary download failed: ${bootstrapResult.reason}. Trying PATH fallback.`);
    void vscode.window.showWarningMessage(
      `Ink: failed to download MCP server binary (${bootstrapResult.reason}). Falling back to PATH or configured command.`
    );
  } else if (bootstrapResult.status === "unsupported_platform") {
    logger.info(`Unsupported platform for auto-download: ${bootstrapResult.reason}`);
  } else if (bootstrapResult.status === "downloaded") {
    void vscode.window.showInformationMessage("Ink: MCP server binary downloaded and ready.");
  }

  // ── MCP client ───────────────────────────────────────────────────────────────
  const mcpServer = settingsService.getMcpServerSettings();
  let mcpClient: SdkMcpClient = buildMcpClient(context, mcpServer, bootstrapService, logger);
  healthService.setMcpClient(mcpClient);

  context.subscriptions.push(logger, eventBus, workspaceService, settingsService, stateService);
  logger.info("Ink extension startup.");

  const mcpConnected = await connectMcpClient(mcpClient, logger, statusBarItem);
  const runtimeFactory = mcpConnected
    ? new McpRuntimeFactory(mcpClient, logger)
    : new MockRuntimeFactory(logger);

  const runtimeManager = new RuntimeManager(
    runtimeFactory,
    workspaceService,
    settingsService,
    healthService,
    stateService,
    eventBus,
    logger
  );

  const dashboardProvider = new DashboardProvider(runtimeManager, stateService, logger);
  const analyticsProvider = new AnalyticsProvider(runtimeManager, stateService, logger);
  const agentsProvider = new AgentsProvider(runtimeManager, stateService, logger);
  const cacheProvider = new CacheProvider(runtimeManager, stateService, logger);
  const graphPanel = new DependencyGraphPanel(logger);
  const optimizePanel = new OptimizePanel(logger);

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider("ink.dashboard", dashboardProvider),
    vscode.window.registerWebviewViewProvider("ink.analytics", analyticsProvider),
    vscode.window.registerTreeDataProvider("ink.agents", agentsProvider),
    vscode.window.registerTreeDataProvider("ink.cache", cacheProvider),
    graphPanel,
    optimizePanel,
    { dispose: () => { void mcpClient.dispose(); } },
    workspaceService.onDidChange(async (workspace) => {
      stateService.setWorkspace(workspace);
      await updateWorkspaceContext(workspace.isOpen);
      await dashboardProvider.refresh();
      await analyticsProvider.refresh();
      agentsProvider.refresh();
      cacheProvider.refresh();
    }),
    settingsService.onDidChange(async () => {
      await dashboardProvider.refresh();
      await analyticsProvider.refresh();
      agentsProvider.refresh();
      cacheProvider.refresh();
    })
  );

  registerCommands(context, runtimeManager, logger, {
    refreshDashboard: () => dashboardProvider.refresh(),
    refreshAnalytics: () => analyticsProvider.refresh(),
    refreshAgents: () => agentsProvider.refresh(),
    refreshCache: () => cacheProvider.refresh()
  }, graphPanel, optimizePanel);

  // ── ink.restartServer ────────────────────────────────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand("ink.restartServer", async () => {
      logger.info("ink.restartServer: restarting MCP server.");
      statusBarItem.text = "$(loading~spin) Ink: restarting…";
      try {
        await mcpClient.dispose();
        mcpClient = buildMcpClient(context, settingsService.getMcpServerSettings(), bootstrapService, logger);
        healthService.setMcpClient(mcpClient);
        const reconnected = await connectMcpClient(mcpClient, logger, statusBarItem);
        runtimeManager.setFactory(
          reconnected
            ? new McpRuntimeFactory(mcpClient, logger)
            : new MockRuntimeFactory(logger)
        );
        await runtimeManager.restart();
        await dashboardProvider.refresh();
        await analyticsProvider.refresh();
        agentsProvider.refresh();
        cacheProvider.refresh();
        void vscode.window.showInformationMessage("Ink MCP server restarted.");
      } catch (err) {
        const inkErr = toInkError(err, "Runtime");
        logger.error("ink.restartServer failed.", inkErr);
        statusBarItem.text = "$(error) Ink: error";
        void vscode.window.showErrorMessage(`Ink: restart failed — ${inkErr.message}`);
      }
    })
  );

  // ── ink.checkForUpdates ──────────────────────────────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand("ink.checkForUpdates", async () => {
      logger.info("ink.checkForUpdates: checking for binary updates.");
      // Remove the cached binary so ensureBinary will re-download.
      const binaryPath = bootstrapService.getBinaryPath();
      if (fs.existsSync(binaryPath)) {
        try {
          fs.rmSync(binaryPath, { force: true });
          logger.info(`ink.checkForUpdates: removed ${binaryPath}, triggering re-download.`);
        } catch (err) {
          const inkErr = toInkError(err, "Runtime");
          logger.error("ink.checkForUpdates: failed to remove binary.", inkErr);
          void vscode.window.showErrorMessage(`Ink: could not remove existing binary — ${inkErr.message}`);
          return;
        }
      }
      const result = await bootstrapService.ensureBinary();
      if (result.status === "downloaded") {
        void vscode.window.showInformationMessage("Ink: MCP server binary updated. Restart server to apply.");
      } else if (result.status === "download_failed") {
        void vscode.window.showErrorMessage(`Ink: update failed — ${result.reason}`);
      } else {
        void vscode.window.showInformationMessage("Ink: binary is up to date.");
      }
    })
  );

  await runtimeManager.start();
  await updateWorkspaceContext(initialWorkspace.isOpen);
}

export function deactivate(): void {
  // Resources are disposed through extension subscriptions.
}

async function updateWorkspaceContext(isOpen: boolean): Promise<void> {
  await vscode.commands.executeCommand("setContext", "ink.workspaceOpen", isOpen);
}

/**
 * Build a fresh MCP client based on current settings.
 *
 * Resolution order for the server command (stdio transport):
 * 1. Explicit `ink.mcpServer.command` setting.
 * 2. Binary bundled with this VSIX at `<extensionPath>/bin/ink_mcp[.exe]`.
 * 3. Binary downloaded by BinaryBootstrapService to global storage.
 * 4. Sibling `mcp/target/{debug,release}/ink_mcp[.exe]` (repo dev).
 * 5. `ink_mcp` on PATH.
 */
function buildMcpClient(
  context: vscode.ExtensionContext,
  mcpServer: ReturnType<SettingsService["getMcpServerSettings"]>,
  bootstrapService: BinaryBootstrapService,
  logger: OutputChannelLogger
): SdkMcpClient {
  if (mcpServer.transport === "http" && mcpServer.url.trim().length > 0) {
    return new SdkMcpClient(logger, {
      transport: "http",
      url: mcpServer.url.trim()
    });
  }
  return new SdkMcpClient(logger, {
    transport: "stdio",
    command: resolveInkMcpCommand(context, mcpServer.command, bootstrapService),
    args: [...mcpServer.args],
    stderr: "pipe"
  });
}

function resolveInkMcpCommand(
  context: vscode.ExtensionContext,
  configured: string,
  bootstrapService: BinaryBootstrapService
): string {
  if (configured.trim().length > 0) {
    return configured.trim();
  }

  const executable = process.platform === "win32" ? "ink_mcp.exe" : "ink_mcp";

  // 1. Bundled binary inside the VSIX (marketplace installs).
  const bundled = path.join(context.extensionPath, "bin", executable);
  if (candidateExists(bundled)) {
    return bundled;
  }

  // 2. Binary downloaded at runtime to global storage by BinaryBootstrapService.
  const globalBin = bootstrapService.getBinaryPath();
  if (candidateExists(globalBin)) {
    return globalBin;
  }

  // 3. Sibling build from the monorepo (dev).
  const base = path.join(context.extensionPath, "..", "mcp", "target");
  for (const profile of ["debug", "release"]) {
    const candidate = path.join(base, profile, executable);
    if (candidateExists(candidate)) {
      return candidate;
    }
  }

  // 4. PATH fallback.
  return "ink_mcp";
}

function candidateExists(filePath: string): boolean {
  try {
    fs.accessSync(filePath);
    return true;
  } catch {
    return false;
  }
}

async function connectMcpClient(
  mcpClient: McpClient,
  logger: OutputChannelLogger,
  statusBarItem: vscode.StatusBarItem
): Promise<boolean> {
  try {
    await mcpClient.connect();
    const servers = await mcpClient.listServers();
    const serverList = servers
      .map((server) => `${server.name} (${server.capabilities.length} capabilities)`)
      .join(", ");
    logger.info(`Ink MCP servers: ${serverList}`);
    statusBarItem.text = "$(check) Ink: connected";
    statusBarItem.tooltip = `Ink MCP Server — ${serverList}\nClick to restart`;
    return true;
  } catch (error) {
    logger.error("Failed to connect to the Ink MCP server. The extension will keep running with mock data.", error);
    statusBarItem.text = "$(warning) Ink: server offline";
    statusBarItem.tooltip = "Ink MCP Server — not connected. Click to restart.";
    return false;
  }
}
