export interface TimelineEvent {
  readonly id: string;
  readonly label: string;
  readonly durationMs: number;
  readonly status: "completed" | "running" | "queued";
}

export interface RuntimeStatistics {
  readonly totalRuns: number;
  readonly averageExecutionTimeMs: number;
  readonly failedRuns: number;
  readonly successfulRuns: number;
}

export interface ExecutionReport {
  readonly generatedAt: Date;
  readonly timeline: readonly TimelineEvent[];
  readonly runtimeStatistics: RuntimeStatistics;
}
