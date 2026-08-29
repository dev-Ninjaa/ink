export type McpMessageDirection = "request" | "response" | "notification";

export interface McpMessage {
  readonly id?: string;
  readonly direction: McpMessageDirection;
  readonly method: string;
  readonly payload: unknown;
  readonly timestamp: Date;
}

export interface McpRequestMessage<TPayload = unknown> extends McpMessage {
  readonly id: string;
  readonly direction: "request";
  readonly payload: TPayload;
}

export interface McpResponseMessage<TPayload = unknown> extends McpMessage {
  readonly id: string;
  readonly direction: "response";
  readonly payload: TPayload;
}

export interface McpNotificationMessage<TPayload = unknown> extends McpMessage {
  readonly direction: "notification";
  readonly payload: TPayload;
}

export interface McpServerDescriptor {
  readonly id: string;
  readonly name: string;
  readonly connected: boolean;
  readonly capabilities: readonly string[];
}
