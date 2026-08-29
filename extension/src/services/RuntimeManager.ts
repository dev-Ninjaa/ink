import {
  BuildDependencyGraphResponse,
  ClearCacheResponse,
  GenerateReportResponse,
  GetAnalyticsResponse,
  GetCacheStatsResponse,
  GetMetricsResponse,
  OptimizeContextResponse,
  RuntimeHealth,
  RuntimeState,
  RuntimeStatus,
  RuntimeVersion,
  ScheduleAgentsResponse
} from "../contracts";
import { EventBus } from "../events/EventBus";
import { AgentSummary } from "../models/Agent";
import { WorkspaceInfo } from "../models/WorkspaceInfo";
import { ExtensionStateService } from "../state/ExtensionStateService";
import { InkSettings, SettingsService } from "./SettingsService";
import { Logger } from "./Logger";
import { ManagedRuntime, Runtime, RuntimeFactory, RuntimeProvider } from "./Runtime";
import { RuntimeHealthService } from "./RuntimeHealthService";
import { WorkspaceService } from "./WorkspaceService";

export class RuntimeManager implements ManagedRuntime, RuntimeProvider {
  private runtime: Runtime;
  private status: RuntimeStatus = {
    state: "stopped",
    message: "Runtime stopped.",
    updatedAt: new Date()
  };
  private version: RuntimeVersion;

  constructor(
    private readonly runtimeFactory: RuntimeFactory,
    private readonly workspaceService: WorkspaceService,
    private readonly settingsService: SettingsService,
    private readonly healthService: RuntimeHealthService,
    private readonly stateService: ExtensionStateService,
    private readonly eventBus: EventBus,
    private readonly logger: Logger
  ) {
    this.version = runtimeFactory.describe();
    this.runtime = this.runtimeFactory.createRuntime();
  }

  getRuntime(): Runtime {
    return this.runtime;
  }

  async start(): Promise<void> {
    this.setStatus("starting", "Runtime starting.");
    this.runtime = this.runtimeFactory.createRuntime();
    this.setStatus("running", "Runtime running.");
    this.eventBus.publish("RuntimeStarted", this.status);
    await this.refreshHealth();
  }

  async stop(): Promise<void> {
    this.setStatus("stopped", "Runtime stopped.");
    this.eventBus.publish("RuntimeStopped", this.status);
    await this.refreshHealth();
  }

  async restart(): Promise<void> {
    await this.stop();
    await this.start();
  }

  async getHealth(): Promise<RuntimeHealth> {
    return this.refreshHealth();
  }

  async getVersion(): Promise<RuntimeVersion> {
    return this.version;
  }

  getStatus(): RuntimeStatus {
    return this.status;
  }

  getWorkspaceInfo(): WorkspaceInfo {
    return this.workspaceService.getWorkspaceInfo();
  }

  getSettings(): InkSettings {
    return this.getValidSettings();
  }

  async getMetrics(): Promise<GetMetricsResponse> {
    return this.runtime.getMetrics({ workspace: this.getWorkspaceInfo() });
  }

  async getAnalytics(): Promise<GetAnalyticsResponse> {
    this.getValidSettings();
    return this.runtime.getAnalytics({ workspace: this.getWorkspaceInfo() });
  }

  async getAgents(): Promise<AgentSummary> {
    return this.runtime.getAgents();
  }

  async getCacheStats(): Promise<GetCacheStatsResponse> {
    this.getValidSettings();
    return this.runtime.getCacheStats({ workspace: this.getWorkspaceInfo() });
  }

  async optimizeContext(): Promise<OptimizeContextResponse> {
    const settings = this.getValidSettings();
    return this.runtime.optimizeContext({
      workspace: this.getWorkspaceInfo(),
      cacheEnabled: settings.enableCache
    });
  }

  async buildDependencyGraph(): Promise<BuildDependencyGraphResponse> {
    return this.runtime.buildDependencyGraph({
      workspace: this.getWorkspaceInfo(),
      includeDevDependencies: true
    });
  }

  async scheduleAgents(): Promise<ScheduleAgentsResponse> {
    const settings = this.getValidSettings();
    return this.runtime.scheduleAgents({
      workspace: this.getWorkspaceInfo(),
      maxAgents: settings.maxAgents,
      parallelismEnabled: settings.enableParallelism
    });
  }

  async generateReport(): Promise<GenerateReportResponse> {
    const settings = this.getValidSettings();
    return this.runtime.generateReport({
      workspace: this.getWorkspaceInfo(),
      analyticsEnabled: settings.enableAnalytics
    });
  }

  async clearCache(): Promise<ClearCacheResponse> {
    return this.runtime.clearCache({ workspace: this.getWorkspaceInfo() });
  }

  private async refreshHealth(): Promise<RuntimeHealth> {
    const health = await this.healthService.checkHealth();
    this.stateService.setHealth(health);
    return health;
  }

  private setStatus(state: RuntimeState, message: string): void {
    this.status = { state, message, updatedAt: new Date() };
    this.stateService.setRuntimeStatus(this.status);
    this.logger.info(message);
  }

  private getValidSettings(): InkSettings {
    return this.settingsService.getSettings();
  }
}
