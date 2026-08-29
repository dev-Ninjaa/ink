export interface OptimizeFile {
  readonly path: string;
  readonly language: string | null;
  readonly tokens: number;
  readonly relevance: number;
  readonly reasons: readonly string[];
}

export interface OptimizeMetrics {
  readonly filesConsidered: number;
  readonly filesSelected: number;
  readonly tokenReductionPercent: number;
}

export interface OptimizeResult {
  readonly query: string;
  readonly selectedFiles: readonly OptimizeFile[];
  readonly droppedFiles: readonly OptimizeFile[];
  readonly tokensBefore: number;
  readonly tokensAfter: number;
  readonly budget: number;
  readonly withinBudget: boolean;
  readonly metrics: OptimizeMetrics;
  readonly warnings: readonly string[];
}
