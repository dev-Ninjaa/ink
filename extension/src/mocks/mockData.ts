import { AgentSummary } from "../models/Agent";
import { CacheStats } from "../models/CacheStats";
import { DependencyGraph } from "../models/DependencyGraph";
import { ExecutionReport } from "../models/ExecutionReport";
import { Metrics } from "../models/Metrics";

export const mockMetrics: Metrics = {
  tokensSaved: 128400,
  cacheHitRate: 82,
  parallelTasks: 7,
  executionTimeMs: 18400,
  contextReductionPercent: 46,
  items: [
    { label: "Tokens Saved", value: "128.4K", trend: "up", detail: "Across recent orchestration runs" },
    { label: "Cache Hit Rate", value: "82%", trend: "up", detail: "Semantic and dependency cache" },
    { label: "Parallel Tasks", value: "7", trend: "flat", detail: "Mock scheduled agents" },
    { label: "Execution Time", value: "18.4s", trend: "down", detail: "Average simulated run time" },
    { label: "Context Reduction", value: "46%", trend: "up", detail: "Compressed workspace context" }
  ]
};

export const mockExecutionReport: ExecutionReport = {
  generatedAt: new Date("2026-08-29T09:30:00.000Z"),
  timeline: [
    { id: "scan", label: "Workspace Scan", durationMs: 2100, status: "completed" },
    { id: "context", label: "Context Optimization", durationMs: 3800, status: "completed" },
    { id: "graph", label: "Dependency Graph", durationMs: 5200, status: "completed" },
    { id: "agents", label: "Agent Scheduling", durationMs: 7300, status: "running" }
  ],
  runtimeStatistics: {
    totalRuns: 24,
    averageExecutionTimeMs: 18400,
    failedRuns: 1,
    successfulRuns: 23
  }
};

export const mockAgents: AgentSummary = {
  active: [
    { id: "agent-review", name: "Review Agent", status: "active", task: "Inspect workspace changes", progressPercent: 64 },
    { id: "agent-cache", name: "Cache Agent", status: "active", task: "Refresh project embeddings", progressPercent: 41 }
  ],
  completed: [
    { id: "agent-map", name: "Graph Agent", status: "completed", task: "Build dependency map", progressPercent: 100 },
    { id: "agent-report", name: "Report Agent", status: "completed", task: "Summarize execution", progressPercent: 100 }
  ],
  pending: [
    { id: "agent-test", name: "Test Agent", status: "pending", task: "Plan targeted verification", progressPercent: 0 }
  ]
};

export const mockCacheStats: CacheStats = {
  hitRate: 82,
  cacheSizeKb: 18432,
  entries: [
    { key: "workspace-symbols", description: "Workspace symbol index", sizeKb: 4096, hits: 38, updatedAt: new Date("2026-08-29T08:45:00.000Z") },
    { key: "dependency-graph", description: "Dependency graph snapshot", sizeKb: 6144, hits: 17, updatedAt: new Date("2026-08-29T08:58:00.000Z") },
    { key: "runtime-profile", description: "Runtime profile summary", sizeKb: 8192, hits: 24, updatedAt: new Date("2026-08-29T09:12:00.000Z") }
  ]
};

export const mockDependencyGraph: DependencyGraph = {
  nodes: [
    { id: "workspace", label: "Workspace", kind: "workspace" },
    { id: "runtime", label: "Runtime", kind: "service" },
    { id: "cache", label: "Cache", kind: "service" },
    { id: "agents", label: "Agents", kind: "agent" }
  ],
  edges: [
    { from: "workspace", to: "runtime", relationship: "provides context" },
    { from: "runtime", to: "cache", relationship: "reads and writes" },
    { from: "runtime", to: "agents", relationship: "schedules" }
  ]
};
