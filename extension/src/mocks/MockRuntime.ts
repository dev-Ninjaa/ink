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
import { Logger } from "../services/Logger";
import { Runtime } from "../services/Runtime";
import { mockAgents, mockCacheStats, mockDependencyGraph, mockExecutionReport, mockMetrics } from "./mockData";

export class MockRuntime implements Runtime {
  constructor(private readonly logger: Logger) {}

  async getMetrics(request: GetMetricsRequest): Promise<GetMetricsResponse> {
    this.logger.info(`Mock runtime metrics requested for ${request.workspace.name}.`);
    return { metrics: mockMetrics };
  }

  async getAnalytics(request: GetAnalyticsRequest): Promise<GetAnalyticsResponse> {
    this.logger.info(`Mock runtime analytics requested for ${request.workspace.name}.`);
    return { report: mockExecutionReport, cache: mockCacheStats };
  }

  async getAgents(): Promise<AgentSummary> {
    this.logger.info("Mock runtime agents requested.");
    return mockAgents;
  }

  async getCacheStats(request: GetCacheStatsRequest): Promise<GetCacheStatsResponse> {
    this.logger.info(`Mock runtime cache stats requested for ${request.workspace.name}.`);
    return { cache: mockCacheStats };
  }

  async optimizeContext(request: OptimizeContextRequest): Promise<OptimizeContextResponse> {
    this.logger.info(`Mock context optimization requested. Cache enabled: ${request.cacheEnabled}.`);
    return { accepted: true };
  }

  async buildDependencyGraph(request: BuildDependencyGraphRequest): Promise<BuildDependencyGraphResponse> {
    this.logger.info(`Mock dependency graph requested. Include dev dependencies: ${request.includeDevDependencies}.`);
    return { graph: mockDependencyGraph };
  }

  async scheduleAgents(request: ScheduleAgentsRequest): Promise<ScheduleAgentsResponse> {
    this.logger.info(`Mock agent scheduling requested. Max agents: ${request.maxAgents}.`);
    return { accepted: true, agents: mockAgents };
  }

  async generateReport(request: GenerateReportRequest): Promise<GenerateReportResponse> {
    this.logger.info(`Mock report generation requested. Analytics enabled: ${request.analyticsEnabled}.`);
    return { report: mockExecutionReport };
  }

  async clearCache(request: ClearCacheRequest): Promise<ClearCacheResponse> {
    this.logger.info(`Mock cache clear requested for ${request.workspace.name}.`);
    return { cleared: true };
  }
}
