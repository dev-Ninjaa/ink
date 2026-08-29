import * as vscode from "vscode";
import { toInkError } from "../errors/InkError";
import { CacheEntry } from "../models/CacheStats";
import { Logger } from "../services/Logger";
import { RuntimeManager } from "../services/RuntimeManager";
import { ExtensionStateService } from "../state/ExtensionStateService";
import { formatDate, formatKilobytes } from "../utils/format";

type CacheTreeNode = WorkspaceNoticeNode | CacheSummaryNode | CacheEntryNode;

interface WorkspaceNoticeNode {
  readonly kind: "notice";
  readonly label: string;
}

interface CacheSummaryNode {
  readonly kind: "summary";
  readonly label: string;
  readonly value: string;
}

interface CacheEntryNode {
  readonly kind: "entry";
  readonly entry: CacheEntry;
}

export class CacheProvider implements vscode.TreeDataProvider<CacheTreeNode> {
  private readonly didChangeTreeDataEmitter = new vscode.EventEmitter<CacheTreeNode | undefined>();
  readonly onDidChangeTreeData = this.didChangeTreeDataEmitter.event;

  constructor(
    private readonly runtimeManager: RuntimeManager,
    private readonly stateService: ExtensionStateService,
    private readonly logger: Logger
  ) {}

  refresh(): void {
    this.didChangeTreeDataEmitter.fire(undefined);
  }

  getTreeItem(element: CacheTreeNode): vscode.TreeItem {
    if (element.kind === "notice") {
      const item = new vscode.TreeItem(element.label, vscode.TreeItemCollapsibleState.None);
      item.iconPath = new vscode.ThemeIcon("folder-opened");
      return item;
    }

    if (element.kind === "summary") {
      const item = new vscode.TreeItem(element.label, vscode.TreeItemCollapsibleState.None);
      item.description = element.value;
      item.iconPath = new vscode.ThemeIcon("database");
      return item;
    }

    const item = new vscode.TreeItem(element.entry.key, vscode.TreeItemCollapsibleState.None);
    item.description = formatKilobytes(element.entry.sizeKb);
    item.tooltip = `${element.entry.description}\nHits: ${element.entry.hits}\nUpdated: ${formatDate(element.entry.updatedAt)}`;
    item.iconPath = new vscode.ThemeIcon("symbol-key");
    return item;
  }

  async getChildren(element?: CacheTreeNode): Promise<CacheTreeNode[]> {
    this.stateService.setSelectedView("cache");
    const workspace = this.runtimeManager.getWorkspaceInfo();
    if (!workspace.isOpen) {
      return element ? [] : [{ kind: "notice", label: "Open a project to use Ink." }];
    }

    if (element) {
      return [];
    }

    try {
      this.runtimeManager.getSettings();
      const { cache } = await this.runtimeManager.getCacheStats();
      return [
        { kind: "summary", label: "Cache Entries", value: String(cache.entries.length) },
        { kind: "summary", label: "Cache Size", value: formatKilobytes(cache.cacheSizeKb) },
        { kind: "summary", label: "Hit Rate", value: `${cache.hitRate}%` },
        ...cache.entries.map((entry) => ({ kind: "entry" as const, entry }))
      ];
    } catch (error) {
      this.logger.error("Cache refresh failed.", toInkError(error, "Runtime"));
      return [{ kind: "notice", label: "Unable to load Ink cache." }];
    }
  }
}
