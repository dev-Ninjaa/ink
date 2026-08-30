import * as fs from "fs";
import * as https from "https";
import * as http from "http";
import * as path from "path";
import { execSync } from "child_process";
import * as vscode from "vscode";
import { Logger } from "./Logger";

const REPO = "dev-Ninjaa/ink";

const PLATFORM_MAP: Record<string, { rust: string; ext: "tar.gz" | "zip" }> = {
  "x64-linux": { rust: "x86_64-unknown-linux-gnu", ext: "tar.gz" },
  "arm64-darwin": { rust: "aarch64-apple-darwin", ext: "tar.gz" },
  "x64-darwin": { rust: "x86_64-apple-darwin", ext: "tar.gz" },
  "x64-win32": { rust: "x86_64-pc-windows-msvc", ext: "zip" },
};

export type BootstrapResult =
  | { readonly status: "already_present"; readonly binaryPath: string }
  | { readonly status: "downloaded"; readonly binaryPath: string }
  | { readonly status: "unsupported_platform"; readonly reason: string }
  | { readonly status: "download_failed"; readonly reason: string }
  | { readonly status: "disabled" };

/**
 * Downloads the platform-native `ink_mcp` binary to VS Code's global storage
 * directory the first time the extension activates. This means users can
 * install the extension directly from the Marketplace without needing to run
 * `npm install ink-mcp` or have Rust/Cargo available.
 *
 * The binary is placed at:
 *   `<globalStoragePath>/bin/ink_mcp[.exe]`
 *
 * The extension's `resolveInkMcpCommand` checks this path before falling back
 * to `ink_mcp` on PATH.
 */
export class BinaryBootstrapService {
  constructor(
    private readonly context: vscode.ExtensionContext,
    private readonly logger: Logger
  ) {}

  /**
   * Returns the path where the bootstrap binary will be placed.
   * The file may or may not exist yet.
   */
  getBinaryPath(): string {
    const executable = process.platform === "win32" ? "ink_mcp.exe" : "ink_mcp";
    return path.join(this.context.globalStorageUri.fsPath, "bin", executable);
  }

  /**
   * Ensures the binary is present. Call once from `activate()`.
   *
   * - If `ink.mcpServer.autoDownload` is false, skips immediately.
   * - If the binary is already present, skips immediately (no version check).
   * - Otherwise downloads and extracts the release asset for this platform.
   */
  async ensureBinary(): Promise<BootstrapResult> {
    const autoDownload = vscode.workspace
      .getConfiguration("ink.mcpServer")
      .get<boolean>("autoDownload", true);

    if (!autoDownload) {
      this.logger.info("BinaryBootstrapService: auto-download disabled via ink.mcpServer.autoDownload.");
      return { status: "disabled" };
    }

    const binaryPath = this.getBinaryPath();

    if (fs.existsSync(binaryPath)) {
      this.logger.info(`BinaryBootstrapService: binary already present at ${binaryPath}`);
      return { status: "already_present", binaryPath };
    }

    const platformKey = `${process.arch}-${process.platform}`;
    const platformEntry = PLATFORM_MAP[platformKey];

    if (!platformEntry) {
      const reason = `Unsupported platform: ${platformKey}. Supported: ${Object.keys(PLATFORM_MAP).join(", ")}`;
      this.logger.info(`BinaryBootstrapService: ${reason}`);
      return { status: "unsupported_platform", reason };
    }

    const version = this.resolveVersion();
    const { rust, ext } = platformEntry;
    const filename = `ink_mcp-${rust}.${ext}`;
    const url = `https://github.com/${REPO}/releases/download/${version}/${filename}`;

    this.logger.info(`BinaryBootstrapService: downloading ${url}`);

    try {
      await vscode.window.withProgress(
        {
          location: vscode.ProgressLocation.Notification,
          title: "Ink: downloading MCP server binary…",
          cancellable: false
        },
        async () => {
          const binDir = path.dirname(binaryPath);
          fs.mkdirSync(binDir, { recursive: true });

          const archivePath = path.join(binDir, filename);
          const buf = await this.download(url);
          fs.writeFileSync(archivePath, buf);

          if (ext === "zip") {
            // Use PowerShell's Expand-Archive — `unzip` is not built-in on Windows.
            execSync(
              `powershell -NoProfile -Command "Expand-Archive -Force -LiteralPath '${archivePath}' -DestinationPath '${binDir}'"`,
              { stdio: "pipe" }
            );
          } else {
            execSync(`tar -xzf "${archivePath}" -C "${binDir}"`, { stdio: "pipe" });
          }

          fs.rmSync(archivePath, { force: true });

          if (process.platform !== "win32") {
            fs.chmodSync(binaryPath, 0o755);
          }
        }
      );

      this.logger.info(`BinaryBootstrapService: binary ready at ${binaryPath}`);
      return { status: "downloaded", binaryPath };
    } catch (err) {
      const reason = err instanceof Error ? err.message : String(err);
      this.logger.info(`BinaryBootstrapService: download failed — ${reason}`);
      return { status: "download_failed", reason };
    }
  }

  private resolveVersion(): string {
    // The extension's own package.json version tracks the server release.
    try {
      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const pkg = require(path.join(this.context.extensionPath, "package.json")) as { version: string };
      return `v${pkg.version}`;
    } catch {
      return "latest";
    }
  }

  private download(url: string): Promise<Buffer> {
    return new Promise((resolve, reject) => {
      const follow = (u: string, depth = 0): void => {
        if (depth > 5) {
          reject(new Error("Too many redirects"));
          return;
        }
        const client = u.startsWith("https") ? https : http;
        client
          .get(u, (res) => {
            const location = res.headers["location"];
            if (res.statusCode !== undefined && [301, 302, 307, 308].includes(res.statusCode) && location) {
              follow(location, depth + 1);
              return;
            }
            if (res.statusCode !== 200) {
              reject(new Error(`HTTP ${res.statusCode ?? "?"} for ${u}`));
              return;
            }
            const chunks: Buffer[] = [];
            res.on("data", (chunk: Buffer) => chunks.push(chunk));
            res.on("end", () => resolve(Buffer.concat(chunks)));
            res.on("error", reject);
          })
          .on("error", reject);
      };
      follow(url);
    });
  }
}
