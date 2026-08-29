import * as path from "path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  const extensionDevelopmentPath = path.resolve(__dirname, "../../");
  const extensionTestsPath = path.resolve(__dirname, "./suite");

  await runTests({
    extensionDevelopmentPath,
    extensionTestsPath,
    version: "1.92.0",
    timeout: 120000
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
