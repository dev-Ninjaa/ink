import { AgentSummary } from "../models/Agent";
import { CacheStats } from "../models/CacheStats";
import { DependencyGraph } from "../models/DependencyGraph";
import { ExecutionReport } from "../models/ExecutionReport";
import { Metrics } from "../models/Metrics";
import { OptimizeResult } from "../models/OptimizeResult";
import { WorkspaceInfo } from "../models/WorkspaceInfo";

export type RuntimeState = "stopped" | "starting" | "running" | "degraded" | "error";

export type HealthState = "healthy" | "degraded" | "unhealthy" | "notConnected" | "unknown";

export interface RuntimeStatus {
  readonly state: RuntimeState;
  readonly message: string;
  readonly updatedAt: Date;
}

export interface RuntimeHealthComponent {
  readonly name: "Runtime" | "MCP" | "Workspace";
  readonly state: HealthState;
  readonly message: string;
  readonly checkedAt: Date;
}

export interface RuntimeHealth {
  readonly overall: HealthState;
  readonly components: readonly RuntimeHealthComponent[];
}

export interface RuntimeVersion {
  readonly name: string;
  readonly version: string;
  readonly build?: string;
}

export interface GetMetricsRequest {
  readonly workspace: WorkspaceInfo;
}

export interface GetMetricsResponse {
  readonly metrics: Metrics;
}

export interface GetAnalyticsRequest {
  readonly workspace: WorkspaceInfo;
}

export interface GetAnalyticsResponse {
  readonly report: ExecutionReport;
  readonly cache: CacheStats;
}

export interface ScheduleAgentsRequest {
  readonly workspace: WorkspaceInfo;
  readonly maxAgents: number;
  readonly parallelismEnabled: boolean;
}

export interface ScheduleAgentsResponse {
  readonly accepted: boolean;
  readonly agents: AgentSummary;
}

export interface BuildDependencyGraphRequest {
  readonly workspace: WorkspaceInfo;
  readonly includeDevDependencies: boolean;
}

export interface BuildDependencyGraphResponse {
  readonly graph: DependencyGraph;
}

export interface GenerateReportRequest {
  readonly workspace: WorkspaceInfo;
  readonly analyticsEnabled: boolean;
}

export interface GenerateReportResponse {
  readonly report: ExecutionReport;
}

export interface OptimizeContextRequest {
  readonly workspace: WorkspaceInfo;
  readonly cacheEnabled: boolean;
}

export interface OptimizeContextResponse {
  readonly accepted: boolean;
  readonly result?: OptimizeResult;
}

export interface GetCacheStatsRequest {
  readonly workspace: WorkspaceInfo;
}

export interface GetCacheStatsResponse {
  readonly cache: CacheStats;
}

export interface ClearCacheRequest {
  readonly workspace: WorkspaceInfo;
}

export interface ClearCacheResponse {
  readonly cleared: boolean;
}
