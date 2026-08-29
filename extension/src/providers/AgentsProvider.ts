import * as vscode from "vscode";
import { toInkError } from "../errors/InkError";
import { Agent, AgentStatus } from "../models/Agent";
import { Logger } from "../services/Logger";
import { RuntimeManager } from "../services/RuntimeManager";
import { ExtensionStateService } from "../state/ExtensionStateService";

type AgentTreeNode = WorkspaceNoticeNode | AgentGroupNode | AgentNode;

interface WorkspaceNoticeNode {
  readonly kind: "notice";
  readonly label: string;
}

interface AgentGroupNode {
  readonly kind: "group";
  readonly label: string;
  readonly status: AgentStatus;
  readonly agents: readonly Agent[];
}

interface AgentNode {
  readonly kind: "agent";
  readonly agent: Agent;
}

export class AgentsProvider implements vscode.TreeDataProvider<AgentTreeNode> {
  private readonly didChangeTreeDataEmitter = new vscode.EventEmitter<AgentTreeNode | undefined>();
  readonly onDidChangeTreeData = this.didChangeTreeDataEmitter.event;

  constructor(
    private readonly runtimeManager: RuntimeManager,
    private readonly stateService: ExtensionStateService,
    private readonly logger: Logger
  ) {}

  refresh(): void {
    this.didChangeTreeDataEmitter.fire(undefined);
  }

  getTreeItem(element: AgentTreeNode): vscode.TreeItem {
    if (element.kind === "notice") {
      const item = new vscode.TreeItem(element.label, vscode.TreeItemCollapsibleState.None);
      item.iconPath = new vscode.ThemeIcon("folder-opened");
      return item;
    }

    if (element.kind === "group") {
      const item = new vscode.TreeItem(`${element.label} (${element.agents.length})`, vscode.TreeItemCollapsibleState.Expanded);
      item.iconPath = new vscode.ThemeIcon(this.getGroupIcon(element.status));
      return item;
    }

    const item = new vscode.TreeItem(element.agent.name, vscode.TreeItemCollapsibleState.None);
    item.description = `${element.agent.progressPercent}%`;
    item.tooltip = element.agent.task;
    item.iconPath = new vscode.ThemeIcon(this.getAgentIcon(element.agent.status));
    return item;
  }

  async getChildren(element?: AgentTreeNode): Promise<AgentTreeNode[]> {
    this.stateService.setSelectedView("agents");
    const workspace = this.runtimeManager.getWorkspaceInfo();
    if (!workspace.isOpen) {
      return element ? [] : [{ kind: "notice", label: "Open a project to use Ink." }];
    }

    if (!element) {
      try {
        this.runtimeManager.getSettings();
        const agents = await this.runtimeManager.getAgents();
        return [
          { kind: "group", label: "Active Agents", status: "active", agents: agents.active },
          { kind: "group", label: "Completed Agents", status: "completed", agents: agents.completed },
          { kind: "group", label: "Pending Agents", status: "pending", agents: agents.pending }
        ];
      } catch (error) {
        this.logger.error("Agents refresh failed.", toInkError(error, "Runtime"));
        return [{ kind: "notice", label: "Unable to load Ink agents." }];
      }
    }

    if (element.kind === "group") {
      return element.agents.map((agent) => ({ kind: "agent", agent }));
    }

    return [];
  }

  private getGroupIcon(status: AgentStatus): string {
    switch (status) {
      case "active":
        return "sync";
      case "completed":
        return "check-all";
      case "pending":
        return "clock";
    }
  }

  private getAgentIcon(status: AgentStatus): string {
    switch (status) {
      case "active":
        return "play";
      case "completed":
        return "check";
      case "pending":
        return "circle-outline";
    }
  }
}
