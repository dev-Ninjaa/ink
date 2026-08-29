export interface MetricItem {
  readonly label: string;
  readonly value: string;
  readonly trend: "up" | "down" | "flat";
  readonly detail: string;
}

export interface Metrics {
  readonly tokensSaved: number;
  readonly cacheHitRate: number;
  readonly parallelTasks: number;
  readonly executionTimeMs: number;
  readonly contextReductionPercent: number;
  readonly items: readonly MetricItem[];
}
