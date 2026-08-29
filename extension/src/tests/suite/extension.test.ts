import * as assert from "assert";
import * as vscode from "vscode";

suite("Ink extension", () => {
  test("is present in the extension host", async () => {
    const extension = vscode.extensions.getExtension("ninja75.ink server");
    assert.ok(extension);
  });
});
