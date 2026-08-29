import * as fs from "fs";
import * as path from "path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({
    color: true,
    ui: "tdd"
  });

  const testsRoot = __dirname;
  for (const file of findTestFiles(testsRoot)) {
    mocha.addFile(file);
  }

  return new Promise((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} tests failed.`));
      } else {
        resolve();
      }
    });
  });
}

function findTestFiles(directory: string): string[] {
  const entries = fs.readdirSync(directory, { withFileTypes: true });
  return entries.flatMap((entry) => {
    const fullPath = path.join(directory, entry.name);
    if (entry.isDirectory()) {
      return findTestFiles(fullPath);
    }

    return entry.isFile() && entry.name.endsWith(".test.js") ? [fullPath] : [];
  });
}
