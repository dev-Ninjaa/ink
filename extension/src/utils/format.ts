export function formatMilliseconds(value: number): string {
  if (value < 1000) {
    return `${value}ms`;
  }

  return `${(value / 1000).toFixed(1)}s`;
}

export function formatKilobytes(value: number): string {
  if (value < 1024) {
    return `${value} KB`;
  }

  return `${(value / 1024).toFixed(1)} MB`;
}

export function formatDate(value: Date): string {
  return new Intl.DateTimeFormat("en", {
    dateStyle: "medium",
    timeStyle: "short"
  }).format(value);
}
