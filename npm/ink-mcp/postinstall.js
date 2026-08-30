#!/usr/bin/env node

const https = require("https");
const http = require("http");
const { execSync } = require("child_process");
const fs = require("fs");
const path = require("path");

const REPO = "dev-Ninjaa/ink";

const PLATFORM_MAP = {
  "x64-linux": { rust: "x86_64-unknown-linux-gnu", ext: "tar.gz" },
  "arm64-darwin": { rust: "aarch64-apple-darwin", ext: "tar.gz" },
  "x64-darwin": { rust: "x86_64-apple-darwin", ext: "tar.gz" },
  "x64-win32": { rust: "x86_64-pc-windows-msvc", ext: "zip" },
};

function getPlatformKey() {
  const arch = process.arch;
  const platform = process.platform;
  const key = `${arch}-${platform}`;
  if (!PLATFORM_MAP[key]) {
    throw new Error(
      `Unsupported platform: ${arch}-${platform}\n` +
        `Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`
    );
  }
  return key;
}

function resolveVersion() {
  if (process.env.INK_MCP_VERSION) return process.env.INK_MCP_VERSION;
  const pkg = require(path.join(__dirname, "package.json"));
  return `v${pkg.version}`;
}

function download(url) {
  return new Promise((resolve, reject) => {
    const client = url.startsWith("https") ? https : http;
    const follow = (u, depth = 0) => {
      if (depth > 5) return reject(new Error("Too many redirects"));
      client
        .get(u, (res) => {
          if ([301, 302, 307, 308].includes(res.statusCode)) {
            return follow(res.headers.location, depth + 1);
          }
          if (res.statusCode !== 200) {
            return reject(new Error(`HTTP ${res.statusCode} for ${u}`));
          }
          const chunks = [];
          res.on("data", (c) => chunks.push(c));
          res.on("end", () => resolve(Buffer.concat(chunks)));
          res.on("error", reject);
        })
        .on("error", reject);
    };
    follow(url);
  });
}

async function main() {
  const key = getPlatformKey();
  const { rust, ext } = PLATFORM_MAP[key];
  const version = resolveVersion();
  const binDir = path.join(__dirname, "bin");
  const executable = process.platform === "win32" ? "ink_mcp.exe" : "ink_mcp";
  const binaryPath = path.join(binDir, executable);

  if (fs.existsSync(binaryPath)) {
    console.log(`@ink/mcp: binary already present at ${binaryPath}`);
    return;
  }

  const filename = `ink_mcp-${rust}.${ext}`;
  const url = `https://github.com/${REPO}/releases/download/${version}/${filename}`;

  console.log(`@ink/mcp: downloading ${url}`);

  fs.mkdirSync(binDir, { recursive: true });
  const archivePath = path.join(binDir, filename);
  const buf = await download(url);
  fs.writeFileSync(archivePath, buf);

  if (ext === "zip") {
    // Use PowerShell's Expand-Archive on Windows (unzip is not built-in)
    execSync(
      `powershell -NoProfile -Command "Expand-Archive -Force -LiteralPath '${archivePath}' -DestinationPath '${binDir}'"`,
      { stdio: "inherit" }
    );
  } else {
    execSync(`tar -xzf "${archivePath}" -C "${binDir}"`, { stdio: "inherit" });
  }

  fs.rmSync(archivePath, { force: true });

  if (process.platform !== "win32") {
    fs.chmodSync(binaryPath, 0o755);
  }

  console.log(`@ink/mcp: ready at ${binaryPath}`);
}

main().catch((err) => {
  console.error(`@ink/mcp: install failed — ${err.message}`);
  console.error("You can set INK_MCP_VERSION to override the release tag.");
  process.exit(1);
});
