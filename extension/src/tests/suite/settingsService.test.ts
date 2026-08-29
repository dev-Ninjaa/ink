import * as assert from "assert";
import * as vscode from "vscode";
import { EventBus } from "../../events/EventBus";
import { SettingsService } from "../../services/SettingsService";

suite("SettingsService", () => {
  test("reads default Ink settings", () => {
    const eventBus = new EventBus();
    const settingsService = new SettingsService(eventBus);

    try {
      const settings = settingsService.getSettings();
      assert.strictEqual(typeof settings.enableCache, "boolean");
      assert.strictEqual(typeof settings.enableAnalytics, "boolean");
      assert.strictEqual(typeof settings.enableParallelism, "boolean");
      assert.strictEqual(typeof settings.maxAgents, "number");
    } finally {
      settingsService.dispose();
      eventBus.dispose();
    }
  });

  test("rejects invalid maxAgents values", () => {
    const eventBus = new EventBus();
    const settingsService = new SettingsService(eventBus);

    try {
      assert.throws(() => settingsService.validate({
        enableCache: true,
        enableAnalytics: true,
        enableParallelism: true,
        maxAgents: 0
      }));
    } finally {
      settingsService.dispose();
      eventBus.dispose();
    }
  });
});
