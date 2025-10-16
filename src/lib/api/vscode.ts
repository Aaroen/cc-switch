import { invoke } from "@tauri-apps/api/core";
import type { CustomEndpoint } from "@/types";
import type { AppType } from "./types";

export interface EndpointLatencyResult {
  url: string;
  latency: number | null;
  status?: number;
  error?: string;
}

export const vscodeApi = {
  async getLiveProviderSettings(appType: AppType) {
    return await invoke("read_live_provider_settings", {
      app_type: appType,
      app: appType,
      appType,
    });
  },

  async testApiEndpoints(
    urls: string[],
    options?: { timeoutSecs?: number }
  ): Promise<EndpointLatencyResult[]> {
    return await invoke("test_api_endpoints", {
      urls,
      timeout_secs: options?.timeoutSecs,
    });
  },

  async getCustomEndpoints(
    appType: AppType,
    providerId: string
  ): Promise<CustomEndpoint[]> {
    return await invoke("get_custom_endpoints", {
      app_type: appType,
      app: appType,
      appType,
      provider_id: providerId,
      providerId,
    });
  },

  async addCustomEndpoint(
    appType: AppType,
    providerId: string,
    url: string
  ): Promise<void> {
    await invoke("add_custom_endpoint", {
      app_type: appType,
      app: appType,
      appType,
      provider_id: providerId,
      providerId,
      url,
    });
  },

  async removeCustomEndpoint(
    appType: AppType,
    providerId: string,
    url: string
  ): Promise<void> {
    await invoke("remove_custom_endpoint", {
      app_type: appType,
      app: appType,
      appType,
      provider_id: providerId,
      providerId,
      url,
    });
  },

  async updateEndpointLastUsed(
    appType: AppType,
    providerId: string,
    url: string
  ): Promise<void> {
    await invoke("update_endpoint_last_used", {
      app_type: appType,
      app: appType,
      appType,
      provider_id: providerId,
      providerId,
      url,
    });
  },

  async exportConfigToFile(filePath: string) {
    return await invoke("export_config_to_file", {
      file_path: filePath,
      filePath,
    });
  },

  async importConfigFromFile(filePath: string) {
    return await invoke("import_config_from_file", {
      file_path: filePath,
      filePath,
    });
  },

  async saveFileDialog(defaultName: string): Promise<string | null> {
    return await invoke("save_file_dialog", {
      default_name: defaultName,
      defaultName,
    });
  },

  async openFileDialog(): Promise<string | null> {
    return await invoke("open_file_dialog");
  },
};
