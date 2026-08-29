export interface DependencyNode {
  readonly id: string;
  readonly label: string;
  readonly kind: "workspace" | "package" | "service" | "agent";
}

export interface DependencyEdge {
  readonly from: string;
  readonly to: string;
  readonly relationship: string;
}

export interface DependencyGraph {
  readonly nodes: readonly DependencyNode[];
  readonly edges: readonly DependencyEdge[];
}
