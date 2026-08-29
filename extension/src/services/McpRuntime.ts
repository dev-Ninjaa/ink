import { AgentSummary } from "../models/Agent";
import {
  BuildDependencyGraphRequest,
  BuildDependencyGraphResponse,
  ClearCacheRequest,
  ClearCacheResponse,
  GenerateReportRequest,
  GenerateReportResponse,
  GetAnalyticsRequest,
  GetAnalyticsResponse,
  GetCacheStatsRequest,
  GetCacheStatsResponse,
  GetMetricsRequest,
  GetMetricsResponse,
  OptimizeContextRequest,
  OptimizeContextResponse,
  ScheduleAgentsRequest,
  ScheduleAgentsResponse
} from "../contracts";
import { Logger } from "./Logger";
import { McpClient } from "./McpClient";
import { Runtime } from "./Runtime";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import {
  AnalysisDocument,
  AgentSummaryDocument,
  CacheStatsDocument,
  ExecutionReportDocument,
  GraphDocument,
  OptimizeDocument,
  ScheduleAgentsDocument,
  agentSummaryToModel,
  analysisToMetrics,
  cacheStatsToModel,
  executionReportToModel,
  graphToModel,
  optimizeToModel
} from "../mcp/mcpMapping";

const SERVER_ID = "ink";

/**
 * `Runtime` backed by the real Ink MCP server.
 *
 * Maps every operation the extension exposes to a server tool. The server
 * keeps process-local orchestration state (agents, cache, run history), so no
 * operation falls back to mock data.
 */
export class McpRuntime implements Runtime {
  constructor(
    private readonly mcpClient: McpClient,
    private readonly logger: Logger
  ) {}

  async getMetrics(request: GetMetricsRequest): Promise<GetMetricsResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Fetching Repository Intelligence for ${root}.`);
    const analysis = await this.invoke<AnalysisDocument>("analyze_repository", { root });
    return { metrics: analysisToMetrics(analysis) };
  }

  async getAnalytics(request: GetAnalyticsRequest): Promise<GetAnalyticsResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Fetching analytics for ${root}.`);
    const analysis = await this.invoke<AnalysisDocument>("analyze_repository", { root });
    const report = await this.invoke<ExecutionReportDocument>("generate_report", {
      root,
      analytics_enabled: true
    });
    return {
      report: executionReportToModel(report),
      cache: await this.cacheStats(request.workspace)
    };
  }

  async getAgents(): Promise<AgentSummary> {
    this.logger.info("Listing scheduled agents from the MCP server.");
    const agents = await this.invoke<AgentSummaryDocument>("list_agents", {});
    return agentSummaryToModel(agents);
  }

  async getCacheStats(request: GetCacheStatsRequest): Promise<GetCacheStatsResponse> {
    this.logger.info(`Fetching cache stats for ${request.workspace.name}.`);
    return { cache: await this.cacheStats(request.workspace) };
  }

  async optimizeContext(request: OptimizeContextRequest): Promise<OptimizeContextResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Optimizing context for ${root}. Cache enabled: ${request.cacheEnabled}.`);
    const document = await this.invoke<OptimizeDocument>("optimize_context", {
      root,
      query: "Prepare repository context for a developer task.",
      max_tokens: 6000
    });
    return { accepted: true, result: optimizeToModel(document) };
  }

  async buildDependencyGraph(request: BuildDependencyGraphRequest): Promise<BuildDependencyGraphResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Building dependency graph for ${root}. Include dev dependencies: ${request.includeDevDependencies}.`);
    const graph = await this.invoke<GraphDocument>("build_dependency_graph", { root });
    return { graph: graphToModel(graph) };
  }

  async scheduleAgents(request: ScheduleAgentsRequest): Promise<ScheduleAgentsResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Scheduling agents for ${root}. Max agents: ${request.maxAgents}; parallel: ${request.parallelismEnabled}.`);
    const result = await this.invoke<ScheduleAgentsDocument>("schedule_agents", {
      root,
      max_agents: request.maxAgents,
      parallelism_enabled: request.parallelismEnabled
    });
    return { accepted: result.accepted, agents: agentSummaryToModel(result.agents) };
  }

  async generateReport(request: GenerateReportRequest): Promise<GenerateReportResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Generating report for ${root}. Analytics enabled: ${request.analyticsEnabled}.`);
    const report = await this.invoke<ExecutionReportDocument>("generate_report", {
      root,
      analytics_enabled: request.analyticsEnabled
    });
    return { report: executionReportToModel(report) };
  }

  async clearCache(request: ClearCacheRequest): Promise<ClearCacheResponse> {
    const root = workspaceRoot(request.workspace);
    this.logger.info(`Clearing cache for ${root}.`);
    await this.invoke<{ cleared: boolean }>("clear_cache", { root });
    return { cleared: true };
  }

  private async cacheStats(workspace: WorkspaceInfo): Promise<ReturnType<typeof cacheStatsToModel>> {
    const root = workspaceRoot(workspace);
    const cache = await this.invoke<CacheStatsDocument>("get_cache_stats", { root });
    return cacheStatsToModel(cache);
  }

  private async invoke<TResponse>(toolName: string, input: unknown): Promise<TResponse> {
    try {
      return await this.mcpClient.invokeTool<TResponse>({ serverId: SERVER_ID, toolName, input });
    } catch (error) {
      this.logger.error(`MCP tool ${toolName} failed.`, error);
      throw error;
    }
  }
}

function workspaceRoot(workspace: WorkspaceInfo): string {
  const root = workspace.path ?? workspace.folders[0]?.path;
  if (!root) {
    throw new Error("Ink requires an open workspace to analyze.");
  }
  return root;
}