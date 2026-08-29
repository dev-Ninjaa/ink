import { RuntimeVersion } from "../contracts";
import { Logger } from "./Logger";
import { McpClient } from "./McpClient";
import { McpRuntime } from "./McpRuntime";
import { Runtime, RuntimeFactory } from "./Runtime";

export class McpRuntimeFactory implements RuntimeFactory {
  constructor(
    private readonly mcpClient: McpClient,
    private readonly logger: Logger
  ) {}

  createRuntime(): Runtime {
    return new McpRuntime(this.mcpClient, this.logger);
  }

  describe(): RuntimeVersion {
    return { name: "InkMcpRuntime", version: "0.1.0" };
  }
}