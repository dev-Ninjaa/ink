#!/usr/bin/env node

const { execFileSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const binDir = __dirname;
const executable = process.platform === "win32" ? "ink_mcp.exe" : "ink_mcp";
const binaryPath = path.join(binDir, executable);

if (!fs.existsSync(binaryPath)) {
  console.error(
    "ink-mcp: binary not found. Run `npm install ink-mcp` to download it."
  );
  process.exit(1);
}

try {
  execFileSync(binaryPath, process.argv.slice(2), { stdio: "inherit" });
} catch (err) {
  process.exit(err.status || 1);
}
