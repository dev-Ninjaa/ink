import * as vscode from "vscode";

export interface Logger {
  info(message: string): void;
  error(message: string, error?: unknown): void;
  dispose(): void;
}

export class OutputChannelLogger implements Logger {
  private readonly channel: vscode.OutputChannel;

  constructor(channelName = "INK") {
    this.channel = vscode.window.createOutputChannel(channelName);
  }

  info(message: string): void {
    this.channel.appendLine(`[info] ${new Date().toISOString()} ${message}`);
  }

  error(message: string, error?: unknown): void {
    this.channel.appendLine(`[error] ${new Date().toISOString()} ${message}`);
    if (error instanceof Error) {
      this.channel.appendLine(error.stack ?? error.message);
      return;
    }
    if (error !== undefined) {
      this.channel.appendLine(String(error));
    }
  }

  dispose(): void {
    this.channel.dispose();
  }
}
