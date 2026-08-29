import { HealthState, RuntimeHealth, RuntimeHealthComponent } from "../contracts";
import { EventBus } from "../events/EventBus";
import { McpClient } from "./McpClient";
import { WorkspaceService } from "./WorkspaceService";

export class RuntimeHealthService {
  constructor(
    private readonly workspaceService: WorkspaceService,
    private readonly eventBus: EventBus,
    private mcpClient?: McpClient
  ) {}

  setMcpClient(mcpClient: McpClient): void {
    this.mcpClient = mcpClient;
  }

  async checkHealth(): Promise<RuntimeHealth> {
    const checkedAt = new Date();
    const workspace = this.workspaceService.getWorkspaceInfo();
    const mcp = await this.checkMcp(checkedAt);
    const components: RuntimeHealthComponent[] = [
      {
        name: "Runtime",
        state: "healthy",
        message: "Healthy",
        checkedAt
      },
      mcp,
      {
        name: "Workspace",
        state: workspace.isOpen ? "healthy" : "degraded",
        message: workspace.isOpen ? "Ready" : "No workspace",
        checkedAt
      }
    ];

    const health: RuntimeHealth = {
      overall: this.getOverallState(components),
      components
    };

    this.eventBus.publish("HealthChanged", health);
    return health;
  }

  private async checkMcp(checkedAt: Date): Promise<RuntimeHealthComponent> {
    if (!this.mcpClient) {
      return {
        name: "MCP",
        state: "notConnected",
        message: "Not Connected",
        checkedAt
      };
    }

    try {
      const servers = await this.mcpClient.listServers();
      const server = servers[0];
      if (!server) {
        return {
          name: "MCP",
          state: "unhealthy",
          message: "No MCP servers registered",
          checkedAt
        };
      }
      return {
        name: "MCP",
        state: "healthy",
        message: `Connected (${server.name})`,
        checkedAt
      };
    } catch {
      return {
        name: "MCP",
        state: "unhealthy",
        message: "Connection failed",
        checkedAt
      };
    }
  }

  private getOverallState(components: readonly RuntimeHealthComponent[]): HealthState {
    if (components.some((component) => component.state === "unhealthy")) {
      return "unhealthy";
    }

    if (components.some((component) => component.state === "degraded")) {
      return "degraded";
    }

    return "healthy";
  }
}