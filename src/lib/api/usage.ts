import { invoke } from "@tauri-apps/api/core";
import type { UsageResult } from "@/types";
import type { AppType } from "./types";

export const usageApi = {
  async query(providerId: string, appType: AppType): Promise<UsageResult> {
    return await invoke("query_provider_usage", {
      provider_id: providerId,
      providerId: providerId,
      app_type: appType,
      app: appType,
      appType,
    });
  },
};
