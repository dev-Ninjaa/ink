import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StreamableHTTPClientTransport } from "@modelcontextprotocol/sdk/client/streamableHttp.js";
import { StdioClientTransport, StdioServerParameters } from "@modelcontextprotocol/sdk/client/stdio.js";
import { McpServerDescriptor } from "../contracts";
import { Logger } from "./Logger";

export interface McpToolInvocation {
  readonly serverId: string;
  readonly toolName: string;
  readonly input: unknown;
}

export type McpConnectionOptions =
  | { readonly transport: "stdio" } & StdioServerParameters
  | { readonly transport: "http"; readonly url: string };

export interface McpClient {
  connect(): Promise<void>;
  listServers(): Promise<readonly McpServerDescriptor[]>;
  invokeTool<TResponse>(invocation: McpToolInvocation): Promise<TResponse>;
  dispose(): Promise<void>;
}

const SERVER_ID = "ink";

/**
 * MCP client backed by the official `@modelcontextprotocol/sdk`.
 *
 * Connects to the `ink_mcp` server either by spawning the binary over stdio
 * or by attaching to a shared Streamable HTTP endpoint. Tool results are JSON
 * documents serialized into a single text content block; `invokeTool` parses
 * that block into `TResponse`.
 */
export class SdkMcpClient implements McpClient {
  private client?: Client;
  private transport?: StdioClientTransport | StreamableHTTPClientTransport;
  private toolNames: readonly string[] = [];

  constructor(
    private readonly logger: Logger,
    private readonly options: McpConnectionOptions
  ) {}

  async connect(): Promise<void> {
    if (this.client) {
      return;
    }

    const transport = this.createTransport();
    const client = new Client(
      { name: "ink-extension", version: "0.1.0" },
      { capabilities: {} }
    );

    this.logger.info(this.connectLogLine());
    await client.connect(transport);

    this.transport = transport;
    this.client = client;
  }

  async listServers(): Promise<readonly McpServerDescriptor[]> {
    await this.ensureConnected();
    return [
      {
        id: SERVER_ID,
        name: "Ink",
        connected: true,
        capabilities: this.toolNames.length > 0 ? this.toolNames : ["analyze_repository", "build_dependency_graph", "optimize_context"]
      }
    ];
  }

  async invokeTool<TResponse>(invocation: McpToolInvocation): Promise<TResponse> {
    await this.ensureConnected();

    const result = await this.client!.callTool({
      name: invocation.toolName,
      arguments: invocation.input as Record<string, unknown>
    });

    const content = (result.content ?? []) as readonly { type?: string; text?: string }[];

    if (result.isError) {
      throw new Error(extractText(content));
    }

    const text = extractText(content);
    if (text.trim().length === 0) {
      return {} as TResponse;
    }
    return JSON.parse(text) as TResponse;
  }

  async dispose(): Promise<void> {
    if (this.transport) {
      await this.transport.close().catch(() => undefined);
      this.transport = undefined;
    }
    this.client = undefined;
    this.toolNames = [];
  }

  private createTransport(): StdioClientTransport | StreamableHTTPClientTransport {
    if (this.options.transport === "http") {
      return new StreamableHTTPClientTransport(new URL(this.options.url));
    }

    const { transport, ...serverParams } = this.options;
    void transport;
    return new StdioClientTransport(serverParams);
  }

  private connectLogLine(): string {
    if (this.options.transport === "http") {
      return `Attaching to Ink MCP server over HTTP: ${this.options.url}`;
    }
    return `Starting Ink MCP server: ${this.options.command} ${this.options.args?.join(" ") ?? ""}`;
  }

  private async ensureConnected(): Promise<void> {
    await this.connect();
    if (this.toolNames.length === 0) {
      const { tools } = await this.client!.listTools();
      this.toolNames = tools.map((tool) => tool.name);
      this.logger.info(`Connected to Ink MCP server. Tools: ${this.toolNames.join(", ")}`);
    }
  }
}

function extractText(content: readonly { type?: string; text?: string }[]): string {
  for (const block of content) {
    if (block.type === "text" && typeof block.text === "string") {
      return block.text;
    }
  }
  return "";
}
