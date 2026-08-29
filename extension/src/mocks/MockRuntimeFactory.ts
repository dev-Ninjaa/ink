import { RuntimeVersion } from "../contracts";
import { Logger } from "../services/Logger";
import { Runtime, RuntimeFactory } from "../services/Runtime";
import { MockRuntime } from "./MockRuntime";

export class MockRuntimeFactory implements RuntimeFactory {
  constructor(private readonly logger: Logger) {}

  createRuntime(): Runtime {
    return new MockRuntime(this.logger);
  }

  describe(): RuntimeVersion {
    return { name: "MockRuntime", version: "0.1.0" };
  }
}
