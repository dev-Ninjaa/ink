export type AgentStatus = "active" | "completed" | "pending";

export interface Agent {
  readonly id: string;
  readonly name: string;
  readonly status: AgentStatus;
  readonly task: string;
  readonly progressPercent: number;
}

export interface AgentSummary {
  readonly active: readonly Agent[];
  readonly completed: readonly Agent[];
  readonly pending: readonly Agent[];
}
