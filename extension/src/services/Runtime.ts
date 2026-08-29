import { AgentSummary } from "../models/Agent";
import { CacheStats } from "../models/CacheStats";
import { DependencyGraph } from "../models/DependencyGraph";
import { ExecutionReport } from "../models/ExecutionReport";
import { Metrics } from "../models/Metrics";
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
  RuntimeHealth,
  RuntimeStatus,
  RuntimeVersion,
  ScheduleAgentsRequest,
  ScheduleAgentsResponse
} from "../contracts";

export interface Runtime {
  getMetrics(request: GetMetricsRequest): Promise<GetMetricsResponse>;
  getAnalytics(request: GetAnalyticsRequest): Promise<GetAnalyticsResponse>;
  getAgents(): Promise<AgentSummary>;
  getCacheStats(request: GetCacheStatsRequest): Promise<GetCacheStatsResponse>;
  optimizeContext(request: OptimizeContextRequest): Promise<OptimizeContextResponse>;
  buildDependencyGraph(request: BuildDependencyGraphRequest): Promise<BuildDependencyGraphResponse>;
  scheduleAgents(request: ScheduleAgentsRequest): Promise<ScheduleAgentsResponse>;
  generateReport(request: GenerateReportRequest): Promise<GenerateReportResponse>;
  clearCache(request: ClearCacheRequest): Promise<ClearCacheResponse>;
}

export interface RuntimeProvider {
  getRuntime(): Runtime;
}

export interface RuntimeFactory {
  createRuntime(): Runtime;
  describe(): RuntimeVersion;
}

export interface ManagedRuntime {
  start(): Promise<void>;
  stop(): Promise<void>;
  restart(): Promise<void>;
  getHealth(): Promise<RuntimeHealth>;
  getVersion(): Promise<RuntimeVersion>;
  getStatus(): RuntimeStatus;
}

export type { AgentSummary, CacheStats, DependencyGraph, ExecutionReport, Metrics };
