import { AgentSummary } from "../models/Agent";
import { CacheEntry, CacheStats } from "../models/CacheStats";
import { DependencyEdge, DependencyGraph, DependencyNode } from "../models/DependencyGraph";
import { ExecutionReport } from "../models/ExecutionReport";
import { Metrics } from "../models/Metrics";
import { OptimizeFile, OptimizeResult } from "../models/OptimizeResult";

/** Loose JSON shapes produced by the engine crates (snake_case serde). */
export interface AnalysisDocument {
  root: string;
  analyzer_version: string;
  summary: {
    files: number;
    directories: number;
    project_roots: number;
    bytes: number;
  };
  performance: {
    scan_duration_ms: number;
    analysis_duration_ms: number;
    total_duration_ms: number;
    files_per_second: number;
  };
  languages: Record<string, number>;
  frameworks: readonly string[];
  entry_points: readonly unknown[];
  modules: readonly unknown[];
  relationships: readonly unknown[];
}

export interface GraphDocument {
  nodes: readonly GraphFileNode[];
  modules: readonly GraphModuleNode[];
  edges: readonly GraphEdgeNode[];
  statistics: { node_count: number; edge_count: number };
}

interface GraphFileNode {
  id: string;
  path: string;
  language?: string | null;
  is_entrypoint: boolean;
}

interface GraphModuleNode {
  id: string;
  name: string;
  kind: string;
}

interface GraphEdgeNode {
  id: string;
  source: string;
  target: string;
  kind: string;
}

export function analysisToMetrics(analysis: AnalysisDocument): Metrics {
  const languageCount = Object.keys(analysis.languages ?? {}).length;
  const totalMs = analysis.performance.total_duration_ms;
  const files = analysis.summary.files;

  return {
    tokensSaved: 0,
    cacheHitRate: 0,
    parallelTasks: (analysis.modules ?? []).length,
    executionTimeMs: totalMs,
    contextReductionPercent: 0,
    items: [
      {
        label: "Files Analyzed",
        value: String(files),
        trend: "up",
        detail: "Repository Intelligence scan"
      },
      {
        label: "Languages",
        value: String(languageCount),
        trend: "flat",
        detail: "Detected languages"
      },
      {
        label: "Entry Points",
        value: String((analysis.entry_points ?? []).length),
        trend: "up",
        detail: "Suggested starting points"
      },
      {
        label: "Modules",
        value: String((analysis.modules ?? []).length),
        trend: "up",
        detail: "Logical modules discovered"
      },
      {
        label: "Analysis Time",
        value: `${(totalMs / 1000).toFixed(1)}s`,
        trend: "down",
        detail: "Total scan + analysis duration"
      }
    ]
  };
}

export function graphToModel(document: GraphDocument): DependencyGraph {
  const nodes: DependencyNode[] = [];
  const edges: DependencyEdge[] = [];

  for (const file of document.nodes ?? []) {
    nodes.push({
      id: file.id,
      label: file.path,
      kind: file.is_entrypoint ? "service" : "package"
    });
  }

  for (const module of document.modules ?? []) {
    nodes.push({ id: module.id, label: module.name, kind: "package" });
  }

  for (const edge of document.edges ?? []) {
    edges.push({ from: edge.source, to: edge.target, relationship: edge.kind });
  }

  return { nodes, edges };
}

/** Loose JSON shapes produced by the orchestration tools (camelCase). */
export interface AgentJson {
  id: string;
  name: string;
  status: string;
  task: string;
  progressPercent: number;
}

export interface AgentSummaryDocument {
  active: readonly AgentJson[];
  completed: readonly AgentJson[];
  pending: readonly AgentJson[];
}

export interface CacheEntryJson {
  key: string;
  description: string;
  sizeKb: number;
  hits: number;
  updatedAt: string;
}

export interface CacheStatsDocument {
  entries: readonly CacheEntryJson[];
  cacheSizeKb: number;
  hitRate: number;
}

export interface ScheduleAgentsDocument {
  accepted: boolean;
  agents: AgentSummaryDocument;
}

export interface ExecutionReportDocument {
  generatedAt: string;
  timeline: readonly { id: string; label: string; durationMs: number; status: string }[];
  runtimeStatistics: {
    totalRuns: number;
    averageExecutionTimeMs: number;
    failedRuns: number;
    successfulRuns: number;
  };
}

export function agentSummaryToModel(document: AgentSummaryDocument): AgentSummary {
  return {
    active: (document.active ?? []).map((agent) => ({
      id: agent.id,
      name: agent.name,
      status: "active",
      task: agent.task,
      progressPercent: agent.progressPercent
    })),
    completed: (document.completed ?? []).map((agent) => ({
      id: agent.id,
      name: agent.name,
      status: "completed",
      task: agent.task,
      progressPercent: agent.progressPercent
    })),
    pending: (document.pending ?? []).map((agent) => ({
      id: agent.id,
      name: agent.name,
      status: "pending",
      task: agent.task,
      progressPercent: agent.progressPercent
    }))
  };
}

export function cacheStatsToModel(document: CacheStatsDocument): CacheStats {
  const entries: CacheEntry[] = (document.entries ?? []).map((entry) => ({
    key: entry.key,
    description: entry.description,
    sizeKb: entry.sizeKb,
    hits: entry.hits,
    updatedAt: new Date(entry.updatedAt)
  }));
  return {
    entries,
    cacheSizeKb: document.cacheSizeKb ?? entries.reduce((sum, entry) => sum + entry.sizeKb, 0),
    hitRate: document.hitRate ?? 0
  };
}

export function executionReportToModel(document: ExecutionReportDocument): ExecutionReport {
  return {
    generatedAt: new Date(document.generatedAt ?? Date.now()),
    timeline: (document.timeline ?? []).map((event) => ({
      id: event.id,
      label: event.label,
      durationMs: event.durationMs,
      status: (event.status === "completed" || event.status === "running" || event.status === "queued"
        ? event.status
        : "queued") as "completed" | "running" | "queued"
    })),
    runtimeStatistics: {
      totalRuns: document.runtimeStatistics?.totalRuns ?? 0,
      averageExecutionTimeMs: document.runtimeStatistics?.averageExecutionTimeMs ?? 0,
      failedRuns: document.runtimeStatistics?.failedRuns ?? 0,
      successfulRuns: document.runtimeStatistics?.successfulRuns ?? 0
    }
  };
}
/** Loose JSON shape produced by the `optimize_context` tool (snake_case). */
export interface OptimizeDocument {
  query: string;
  selected: readonly OptimizeFileJson[];
  dropped: readonly OptimizeFileJson[];
  tokens: { tokens_before: number; tokens_after: number; budget: number; within_budget: boolean };
  metrics: {
    files_considered: number;
    files_selected: number;
    token_reduction_percent: number;
  };
  warnings: readonly string[];
}

interface OptimizeFileJson {
  path: string;
  language?: string | null;
  tokens: number;
  relevance: number;
  reasons?: readonly string[];
}

export function optimizeToModel(document: OptimizeDocument): OptimizeResult {
  const toFile = (file: OptimizeFileJson): OptimizeFile => ({
    path: file.path,
    language: file.language ?? null,
    tokens: file.tokens,
    relevance: file.relevance,
    reasons: file.reasons ?? []
  });

  return {
    query: document.query,
    selectedFiles: (document.selected ?? []).map(toFile),
    droppedFiles: (document.dropped ?? []).map(toFile),
    tokensBefore: document.tokens?.tokens_before ?? 0,
    tokensAfter: document.tokens?.tokens_after ?? 0,
    budget: document.tokens?.budget ?? 0,
    withinBudget: document.tokens?.within_budget ?? true,
    metrics: {
      filesConsidered: document.metrics?.files_considered ?? 0,
      filesSelected: document.metrics?.files_selected ?? 0,
      tokenReductionPercent: document.metrics?.token_reduction_percent ?? 0
    },
    warnings: document.warnings ?? []
  };
}
