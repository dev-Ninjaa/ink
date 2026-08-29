export interface CacheEntry {
  readonly key: string;
  readonly description: string;
  readonly sizeKb: number;
  readonly hits: number;
  readonly updatedAt: Date;
}

export interface CacheStats {
  readonly entries: readonly CacheEntry[];
  readonly cacheSizeKb: number;
  readonly hitRate: number;
}
