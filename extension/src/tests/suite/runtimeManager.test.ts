import * as assert from "assert";
import * as vscode from "vscode";
import { EventBus } from "../../events/EventBus";
import { MockRuntimeFactory } from "../../mocks/MockRuntimeFactory";
import { OutputChannelLogger } from "../../services/Logger";
import { RuntimeHealthService } from "../../services/RuntimeHealthService";
import { RuntimeManager } from "../../services/RuntimeManager";
import { SettingsService } from "../../services/SettingsService";
import { WorkspaceService } from "../../services/WorkspaceService";
import { ExtensionStateService } from "../../state/ExtensionStateService";

suite("RuntimeManager", () => {
  test("starts, stops, and reports mock version", async () => {
    const disposables: vscode.Disposable[] = [];
    const eventBus = new EventBus();
    const logger = new OutputChannelLogger("INK Tests");
    const workspaceService = new WorkspaceService(eventBus);
    const settingsService = new SettingsService(eventBus);
    const initialWorkspace = workspaceService.getWorkspaceInfo();
    const stateService = new ExtensionStateService({
      runtimeStatus: { state: "stopped", message: "Runtime stopped.", updatedAt: new Date() },
      workspace: initialWorkspace,
      selectedView: "dashboard"
    });
    const healthService = new RuntimeHealthService(workspaceService, eventBus);
    const manager = new RuntimeManager(
      new MockRuntimeFactory(logger),
      workspaceService,
      settingsService,
      healthService,
      stateService,
      eventBus,
      logger
    );

    disposables.push(eventBus, logger, workspaceService, settingsService, stateService);

    try {
      await manager.start();
      assert.strictEqual(manager.getStatus().state, "running");
      assert.strictEqual((await manager.getVersion()).name, "MockRuntime");

      await manager.stop();
      assert.strictEqual(manager.getStatus().state, "stopped");
    } finally {
      for (const disposable of disposables) {
        disposable.dispose();
      }
    }
  });
});
