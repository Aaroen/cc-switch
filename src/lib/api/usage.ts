import { invoke } from "@tauri-apps/api/core";
import type { UsageResult } from "@/types";
import type { AppId } from "./types";

export const usageApi = {
  async query(providerId: string, appId: AppId): Promise<UsageResult> {
    return await invoke("query_provider_usage", {
      provider_id: providerId,
      app: appId,
    });
  },
};
